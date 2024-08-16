use crate::storage::tls_client_hello::StoredTlsClientHelloMetadata;
use anyhow::Result;
use clickhouse::inserter::Inserter;
use clickhouse::Client;
use std::time::Duration;

pub(crate) mod tls_client_hello;

pub(crate) struct ClickhouseConnection {
    client: Client,
}

impl ClickhouseConnection {
    pub(crate) fn new() -> Self {
        let client = Client::default()
            .with_url("http://localhost:8123")
            .with_user("clickhouse")
            .with_password("clickhouse")
            .with_database("default");
        Self { client }
    }

    /// TODO: Evaluate and replace with a
    /// proper migration tool for clickhouse
    pub(crate) async fn initialize_db(&self) -> Result<()> {
        self.client
            .query(
                r#"
    CREATE TABLE IF NOT EXISTS tls_client_hello
    (
        uuid UUID,
        src_ip IPv6,
        src_asn UInt32 CODEC(ZSTD),
        src_asn_handle LowCardinality(String) CODEC(ZSTD),
        src_asn_description LowCardinality(String) CODEC(ZSTD),
        dst_ip IPv6,
        dst_asn UInt32 CODEC(ZSTD),
        dst_asn_handle LowCardinality(String) CODEC(ZSTD),
        dst_asn_description LowCardinality(String) CODEC(ZSTD),
        src_port UInt16,
        dst_port UInt16,
        outer_version UInt16,
        inner_version UInt16,
        ciphers Array(UInt16),
        extensions Array(UInt16),
        sni LowCardinality(String),
        ec_curves Array(UInt16),
        ec_curve_point_formats Array(UInt8),
        ja3 LowCardinality(String)
    )
    ENGINE = ReplacingMergeTree
    ORDER BY uuid;
    "#,
            )
            .execute()
            .await?;
        Ok(())
    }

    pub(crate) fn tls_client_hello_inserter(
        &self,
    ) -> Result<Inserter<StoredTlsClientHelloMetadata>> {
        Ok(self
            .client
            .inserter("tls_client_hello")?
            .with_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(20)))
            .with_max_bytes(1024 * 1024 * 256)
            .with_period(Some(Duration::from_secs(1))))
    }
}
