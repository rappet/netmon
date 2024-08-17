// This file is @generated by prost-build.
/// Send when a packet is sampled by the shild
///
/// Kafka topic is chosen depending on sample reason
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PacketSample {
    #[prost(uint64, tag = "1")]
    pub timestamp_ms: u64,
    /// Use the raw packet, so we can switch out analytics in the control plane
    #[prost(bytes = "bytes", tag = "2")]
    pub ip_packet_raw: ::prost::bytes::Bytes,
}
