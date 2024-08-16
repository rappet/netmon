mod opts;

use std::{
    io::ErrorKind,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use clap::Parser;
use kafka_model::{
    packet_sample::PacketSample,
    topics::{TCP_ACK, TLS_CLIENT_HELLO},
};
use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::{IpNextHeaderProtocol, IpNextHeaderProtocols},
    ipv4::Ipv4Packet,
    ipv6::Ipv6Packet,
    tcp::{TcpFlags, TcpPacket},
    Packet,
};
use pnet_datalink::{pcap, Channel};
use prost::Message;
use rdkafka::{
    producer::{BaseRecord, DefaultProducerContext, Producer, ThreadedProducer},
    ClientConfig,
};
use tls_parser::{TlsMessage, TlsMessageHandshake};
use tracing::debug;

use crate::opts::Opts;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    debug!("connecting to Kafka");

    let producer: &ThreadedProducer<DefaultProducerContext> = &ClientConfig::new()
        .set("bootstrap.servers", &opts.kafka_bootstrap_servers)
        .set(
            "message.timeout.ms",
            opts.kafka_message_timeout_ms.to_string(),
        )
        .set("batch.size", "100000")
        .set("linger.ms", "100")
        .set("compression.type", "lz4")
        .set("acks", "1")
        .create()
        .expect("Producer creation error");

    let mut packet_receiver = match pcap::from_file(&opts.pcap_file, pcap::Config::default())? {
        Channel::Ethernet(_sender, receiver) => receiver,
        _ => bail!("pcap channel does not contain receiver"),
    };

    // TODO: The used crate to parse pcaps has many shortcomings (no timestamp extraction, bad EOF handling, ...)

    debug!("starting extraction");

    while let Some(packet) = packet_receiver.next().map(Some).or_else(|err| {
        match (err.kind(), err.to_string().as_str()) {
            // We don't have better EOF detection :(
            (ErrorKind::Other, "no more packets to read from the file") => Ok(None),
            _ => Err(err).context("reading PCAP packet"),
        }
    })? {
        if let Some((extraction_type, ip_packet)) = handle_ethernet_frame(packet) {
            // TODO: In reality, we want the time in the PCAP here
            let timestamp_ms: u64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("duration since unix epoch does always work")
                .as_millis()
                .try_into()
                .context("UNIX time millis does not fit in u32")?;
            let timestamp_ms_string = timestamp_ms.to_string();

            let topic = match extraction_type {
                ExtractionType::TcpAck => TCP_ACK,
                ExtractionType::TlsClientHello => TLS_CLIENT_HELLO,
            };

            let payload = PacketSample {
                timestamp_ms,
                ip_packet_raw: ip_packet,
                ..PacketSample::default()
            };
            let encoded_payload = payload.encode_to_vec();
            let delivery_status = producer.send(
                BaseRecord::<str, [u8]>::to(topic)
                    .key(&timestamp_ms_string)
                    .payload(&encoded_payload),
            );
            debug!(?delivery_status, "detected TCP ACK packet");
        }
    }

    producer.flush(Duration::from_secs(10))?;

    Ok(())
}

enum ExtractionType {
    TcpAck,
    TlsClientHello,
}

fn handle_ethernet_frame(frame: &[u8]) -> Option<(ExtractionType, Bytes)> {
    if let Some(ethernet_packet) = EthernetPacket::new(frame) {
        let should_extract = match ethernet_packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                if let Some(ipv4_packet) = Ipv4Packet::new(ethernet_packet.payload()) {
                    let next_header_protocol = ipv4_packet.get_next_level_protocol();
                    handle_ip_payload(next_header_protocol, ipv4_packet.payload())
                } else {
                    None
                }
            }
            EtherTypes::Ipv6 => {
                if let Some(ipv6_packet) = Ipv6Packet::new(ethernet_packet.payload()) {
                    let next_header_protocol = ipv6_packet.get_next_header();
                    handle_ip_payload(next_header_protocol, ipv6_packet.payload())
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(extraction_type) = should_extract {
            let payload = Bytes::copy_from_slice(ethernet_packet.payload());
            return Some((extraction_type, payload));
        }
    }
    None
}
fn handle_ip_payload(
    next_header_protocol: IpNextHeaderProtocol,
    ip_payload: &[u8],
) -> Option<ExtractionType> {
    if next_header_protocol == IpNextHeaderProtocols::Tcp {
        if let Some(tcp_packet) = TcpPacket::new(ip_payload) {
            let flags = tcp_packet.get_flags();
            if flags == TcpFlags::ACK && tcp_packet.payload().is_empty() {
                return Some(ExtractionType::TcpAck);
            } else if [
                TcpFlags::SYN,
                TcpFlags::SYN | TcpFlags::ACK,
                TcpFlags::FIN,
                TcpFlags::RST,
            ]
            .contains(&flags)
            {
                return None;
            } else {
                if let Ok((_rest, plaintext)) =
                    tls_parser::parse_tls_plaintext(tcp_packet.payload())
                {
                    if plaintext.msg.iter().any(|message| {
                        matches!(
                            message,
                            TlsMessage::Handshake(TlsMessageHandshake::ClientHello(_))
                        )
                    }) {
                        return Some(ExtractionType::TlsClientHello);
                    }
                }
            }
        }
    }
    None
}
