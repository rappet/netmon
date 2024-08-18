// This file is @generated by prost-build.
/// Scraped PTR record
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DnsReverseRecord {
    #[prost(uint32, tag = "1")]
    pub timestamp_seconds: u32,
    /// / IP address, IPv6 mapped in case of IPv4
    #[prost(bytes = "bytes", tag = "2")]
    pub ip_addr: ::prost::bytes::Bytes,
    #[prost(string, repeated, tag = "3")]
    pub ptr_records: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// true if no record was available
    #[prost(bool, tag = "4")]
    pub no_record: bool,
}
