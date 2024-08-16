use clickhouse::Row;
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use uuid::Uuid;

#[derive(Row, Serialize, Deserialize, Debug, PartialEq)]
pub struct StoredTlsClientHelloMetadata {
    #[serde(with = "clickhouse::serde::uuid")]
    pub uuid: Uuid,
    pub src_ip: Ipv6Addr,
    pub src_asn: u32,
    pub src_asn_handle: String,
    pub src_asn_description: String,
    pub dst_ip: Ipv6Addr,
    pub dst_asn: u32,
    pub dst_asn_handle: String,
    pub dst_asn_description: String,
    pub src_port: u16,
    pub dst_port: u16,
    /// Version of the outer TLS header
    pub outer_version: u16,
    /// Version in the client hello
    pub inner_version: u16,
    pub ciphers: Vec<u16>,
    pub extensions: Vec<u16>,
    pub sni: String,
    pub ec_curves: Vec<u16>,
    pub ec_curve_point_formats: Vec<u8>,
    pub ja3: String,
}
