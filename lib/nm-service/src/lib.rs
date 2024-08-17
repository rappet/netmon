mod producer;

#[cfg(feature = "clickhouse")]
pub use clickhouse;
#[cfg(feature = "kafka")]
pub use kafka_model;
#[cfg(feature = "kafka")]
pub use rdkafka;

#[cfg(test)]
mod tests {
    use super::*;
}
