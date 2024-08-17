pub use clap;
#[cfg(feature = "clickhouse")]
pub use clickhouse;
#[cfg(feature = "kafka")]
pub use kafka_model;
#[cfg(feature = "kafka")]
pub use rdkafka;

mod opts;
#[cfg(feature = "producer")]
pub mod producer;

use anyhow::Result;
pub use opts::LibOpts;

use crate::producer::Producer;

pub struct NMService {
    opts: LibOpts,
}

impl NMService {
    pub fn init(opts: &LibOpts) -> Self {
        tracing_subscriber::fmt::init();

        NMService { opts: opts.clone() }
    }

    pub async fn create_producer(&self, topic: &str) -> Result<Producer> {
        Producer::new(self, topic).await
    }
}
