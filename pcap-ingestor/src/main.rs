mod opts;

use std::{
    io::ErrorKind,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use kafka_model::{
    packet_sample::PacketSample,
    topics::{TCP_ACK, TLS_CLIENT_HELLO},
};
use nm_service::{clap::Parser, NMService};
use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::{IpNextHeaderProtocol, IpNextHeaderProtocols},
    ipv4::Ipv4Packet,
    ipv6::Ipv6Packet,
    tcp::{TcpFlags, TcpPacket},
    Packet,
};
use pnet_datalink::{pcap, Channel};
use tls_parser::{TlsMessage, TlsMessageHandshake};
use tracing::debug;

use crate::opts::Opts;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let svc = NMService::init(&opts.lib_opts);

    let tcp_ack_producer = svc.create_producer(TCP_ACK).await?;
    let tls_client_hello_producer = svc.create_producer(TLS_CLIENT_HELLO).await?;

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

            let producer = match extraction_type {
                ExtractionType::TcpAck => &tcp_ack_producer,
                ExtractionType::TlsClientHello => &tls_client_hello_producer,
            };

            let payload = PacketSample {
                timestamp_ms,
                ip_packet_raw: ip_packet,
                ..PacketSample::default()
            };
            producer.send(&timestamp_ms_string, &payload).await?;
        }
    }

    // TODO: flush

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
            } else if let Ok((_rest, plaintext)) =
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
    None
}
