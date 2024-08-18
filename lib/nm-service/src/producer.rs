//! Module for sending data to Kafka topics, files or just stdout

use std::fmt::Debug;

use anyhow::{Context, Result};
#[cfg(feature = "kafka")]
use rdkafka::{
    producer::{BaseRecord, DefaultProducerContext, ThreadedProducer},
    ClientConfig,
};
use serde::Serialize;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tracing::{debug, info, warn};

use crate::{opts::ProducerType, NMService};

pub struct Producer {
    inner: ProducerImplementation,
}

impl Producer {
    pub(crate) async fn new(nm_service: &NMService, topic: &str) -> Result<Self> {
        let inner = match nm_service.opts.producer_type {
            ProducerType::Log => {
                warn!(
                    topic,
                    "created new producer in log mode, please set log level at least to debug"
                );
                ProducerImplementation::Log {
                    topic: topic.to_owned(),
                }
            }
            ProducerType::File => ProducerImplementation::File {
                writer: {
                    let path = format!("streams/{topic}.json");
                    Mutex::new(BufWriter::new(File::create_new(&path).await.with_context(
                        || format!("failed creating producer output file {path:?}"),
                    )?))
                },
            },
            #[cfg(feature = "kafka")]
            ProducerType::Kafka => {
                ProducerImplementation::Kafka(KafkaProducer::new(nm_service, topic)?)
            }
        };

        Ok(Self { inner })
    }

    pub async fn send<M>(&self, key: &str, message: &M) -> Result<()>
    where
        M: Debug + Serialize + prost::Message,
    {
        match &self.inner {
            ProducerImplementation::Log { topic } => {
                debug!(topic, key, "{message:?}");
                Ok(())
            }
            ProducerImplementation::File { writer } => {
                let mut guard = writer.lock().await;
                let mut serialized =
                    simd_json::to_string(message).context("failed serializing message")?;
                serialized.push('\n');
                guard.write_all(serialized.as_bytes()).await?;
                Ok(())
            }
            #[cfg(feature = "kafka")]
            ProducerImplementation::Kafka(kafka) => {
                kafka.send(key.as_ref(), &message.encode_to_vec()).await
            }
        }
    }

    pub async fn flush(&self) -> Result<()> {
        match &self.inner {
            ProducerImplementation::Log { .. } => Ok(()),
            ProducerImplementation::File { writer } => {
                writer.lock().await.flush().await?;
                Ok(())
            }
            #[cfg(feature = "kafka")]
            ProducerImplementation::Kafka(producer) => producer.flush().await,
        }
    }
}

pub enum ProducerImplementation {
    Log {
        topic: String,
    },
    File {
        writer: Mutex<BufWriter<File>>,
    },
    #[cfg(feature = "kafka")]
    Kafka(KafkaProducer),
}

#[cfg(feature = "kafka")]
pub struct KafkaProducer {
    topic: String,
    producer: ThreadedProducer<DefaultProducerContext>,
}

#[cfg(feature = "kafka")]
impl KafkaProducer {
    fn new(nm_service: &NMService, topic: &str) -> Result<Self> {
        let opts = &nm_service.opts;
        let producer = ClientConfig::new()
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
            .context("Kafka producer creation error")?;

        info!(topic, boostrap_servers = ?opts.kafka_bootstrap_servers, "registered new Kafka producer");
        Ok(Self {
            topic: topic.to_owned(),
            producer,
        })
    }

    async fn send(&self, key: &str, payload: &[u8]) -> Result<()> {
        let _delivery_status = self.producer.send(
            BaseRecord::<str, [u8]>::to(&self.topic)
                .key(key)
                .payload(payload),
        );
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.flush().await?;
        Ok(())
    }
}
