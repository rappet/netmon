use std::path::PathBuf;

use nm_service::{clap, clap::Parser, LibOpts};

/// An evaluation/testing/debugging tool that
/// extracts certain packets from a pcap and sends them to Kafka
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Opts {
    #[arg(env)]
    pub(crate) pcap_file: PathBuf,

    #[command(flatten)]
    pub lib_opts: LibOpts,
}
