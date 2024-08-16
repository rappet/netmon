use clap::Parser;
use std::path::PathBuf;

/// An evaluation/testing/debugging tool that
/// extracts certain packets from a pcap and sends them to Kafka
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Opts {
    /// Comma separated list of bootstrap servers
    #[arg(long, env, default_value = "localhost:9094")]
    pub(crate) kafka_bootstrap_servers: String,

    #[arg(long, env, default_value = "5000")]
    pub(crate) kafka_message_timeout_ms: u32,

    #[arg(env)]
    pub(crate) pcap_file: PathBuf,
}
