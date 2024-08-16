use std::net::IpAddr;

use bytes::Bytes;
use pnet::packet::{
    ip::IpNextHeaderProtocols, ipv4::Ipv4Packet, ipv6::Ipv6Packet, tcp::TcpPacket, Packet,
};

#[derive(Debug, PartialEq)]
pub struct TcpMetadata {
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
}

impl TcpMetadata {
    pub fn parse_ip_packet(ip_packet: &Bytes) -> Option<(TcpMetadata, Bytes)> {
        let proto = ip_packet.first()? & 0xF0;
        let (src_ip, dst_ip, l4_protocol, ip_payload) = match proto {
            0x40 => {
                let ipv4 = Ipv4Packet::new(ip_packet)?;
                (
                    ipv4.get_source().into(),
                    ipv4.get_destination().into(),
                    ipv4.get_next_level_protocol(),
                    ip_packet.slice_ref(ipv4.payload()),
                )
            }
            0x60 => {
                let ipv6 = Ipv6Packet::new(ip_packet)?;
                (
                    ipv6.get_source().into(),
                    ipv6.get_destination().into(),
                    ipv6.get_next_header(),
                    ip_packet.slice_ref(ipv6.payload()),
                )
            }
            _ => return None,
        };

        if l4_protocol != IpNextHeaderProtocols::Tcp {
            return None;
        }

        let tcp_packet = TcpPacket::new(&ip_payload)?;

        let src_port = tcp_packet.get_source();
        let dst_port = tcp_packet.get_destination();

        let payload = ip_packet.slice_ref(tcp_packet.payload());

        Some((
            Self {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
            },
            payload,
        ))
    }
}
