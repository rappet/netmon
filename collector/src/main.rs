use std::net::{IpAddr, Ipv6Addr};

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Timelike, Utc};
use clap::Parser;
use ip_lookup::IpDatabase;
use nm_service::{
    kafka_model::{
        packet_sample::PacketSample,
        topics::{TCP_ACK, TLS_CLIENT_HELLO},
    },
    rdkafka::{
        config::RDKafkaLogLevel,
        consumer::{Consumer, StreamConsumer},
        ClientConfig, Message,
    },
};
use prost::Message as _;
use tracing::{error, info, warn};
use uuid::{NoContext, Uuid};

use crate::{
    extractors::{tcp_header::TcpMetadata, tls_client_hello::extract_client_hello},
    opts::Opts,
    storage::{tls_client_hello::StoredTlsClientHelloMetadata, ClickhouseConnection},
};

mod extractors;
mod opts;
mod storage;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "test")
        .set("bootstrap.servers", &opts.kafka_bootstrap_servers)
        .set("enable.partition.eof", "false")
        .set(
            "session.timeout.ms",
            &opts.kafka_session_timeout_ms.to_string(),
        )
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create()?;

    consumer.subscribe(&[TCP_ACK, TLS_CLIENT_HELLO])?;

    let clickhouse = ClickhouseConnection::new();
    clickhouse.initialize_db().await?;

    let ip_lookup = IpDatabase::from_static();

    let mut inserter = clickhouse.tls_client_hello_inserter()?;

    loop {
        match consumer.recv().await {
            Err(e) => error!("Kafka error: {}", e),
            Ok(m) => {
                if let Some((_encoded_key, encoded_message)) = m.key().zip(m.payload()) {
                    if let Ok(message) = PacketSample::decode(encoded_message) {
                        match m.topic() {
                            TLS_CLIENT_HELLO => {
                                if let Some(client_hello) = extract_tcp_client_hello(
                                    &ip_lookup,
                                    DateTime::from_timestamp_millis(
                                        message.timestamp_ms.try_into()?,
                                    )
                                    .context("can't convert timestamp to unix time")?,
                                    &message.ip_packet_raw,
                                ) {
                                    info!("{client_hello:?}");
                                    inserter.write(&client_hello)?;
                                    inserter.commit().await?;
                                };
                            }
                            _ => {}
                        }
                        //info!(?message, "got packet");
                    } else {
                        warn!("decoding error");
                    }
                }
            }
        };
    }

    //let stats = inserter.commit().await?;
    //println!("{stats:?}");
}

fn extract_tcp_client_hello(
    ip_lookup: &IpDatabase,
    timestamp: DateTime<Utc>,
    ip_payload: &Bytes,
) -> Option<StoredTlsClientHelloMetadata> {
    let (l4_payload, tcp_metadata) = TcpMetadata::parse_ip_packet(ip_payload)?;
    let tls_client_hello_metadata = extract_client_hello(&tcp_metadata)?;

    let ja3 = tls_client_hello_metadata.ja3();

    let src_as_meta = ip_lookup.lookup(l4_payload.src_ip);
    let dst_as_meta = ip_lookup.lookup(l4_payload.dst_ip);

    Some(StoredTlsClientHelloMetadata {
        uuid: Uuid::new_v7(uuid::Timestamp::from_unix(
            NoContext,
            timestamp.timestamp().try_into().ok()?,
            timestamp.nanosecond(),
        )),
        src_ip: ip_to_stored_ipv6(l4_payload.src_ip),
        src_asn: src_as_meta.map(|meta| meta.asn).unwrap_or_default(),
        src_asn_handle: src_as_meta
            .and_then(|meta| meta.handle.clone())
            .unwrap_or_default(),
        src_asn_description: src_as_meta
            .and_then(|meta| meta.description.clone())
            .unwrap_or_default(),
        dst_ip: ip_to_stored_ipv6(l4_payload.dst_ip),
        dst_asn: dst_as_meta.map(|meta| meta.asn).unwrap_or_default(),
        dst_asn_handle: dst_as_meta
            .and_then(|meta| meta.handle.clone())
            .unwrap_or_default(),
        dst_asn_description: dst_as_meta
            .and_then(|meta| meta.description.clone())
            .unwrap_or_default(),
        src_port: l4_payload.src_port,
        dst_port: l4_payload.dst_port,
        outer_version: tls_client_hello_metadata.outer_version,
        inner_version: tls_client_hello_metadata.inner_version,
        ciphers: tls_client_hello_metadata.ciphers,
        extensions: tls_client_hello_metadata.extensions,
        sni: tls_client_hello_metadata.sni,
        ec_curves: tls_client_hello_metadata.ec_curves,
        ec_curve_point_formats: tls_client_hello_metadata.ec_curve_point_formats,
        ja3,
    })
}

fn ip_to_stored_ipv6(ip: IpAddr) -> Ipv6Addr {
    match ip {
        IpAddr::V4(v4) => v4.to_ipv6_mapped(),
        IpAddr::V6(v6) => v6,
    }
}
