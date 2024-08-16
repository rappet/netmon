use clap::Parser;

/// Collects samples from Kafka and extracts metadata to Clickhouse
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Opts {
    /// Comma separated list of bootstrap servers
    #[arg(long, env, default_value = "localhost:9094")]
    pub(crate) kafka_bootstrap_servers: String,

    #[arg(long, env, default_value = "6000")]
    pub(crate) kafka_session_timeout_ms: u32,
}
