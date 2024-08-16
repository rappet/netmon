use std::{
    collections::{BTreeMap, HashMap},
    net::IpAddr,
    sync::Arc,
    thread,
    thread::{spawn, JoinHandle},
};

use ipnet::{Ipv4Net, Ipv6Net};
use prefix_trie::PrefixMap;
use serde::{Deserialize, Serialize};

const IPV4_TO_ASN_RAW_BR: &[u8] = include_bytes!("ipv4_to_asn.bin.br");
const IPV6_TO_ASN_RAW_BR: &[u8] = include_bytes!("ipv6_to_asn.bin.br");
const ASN_METADATA_RAW_BR: &[u8] = include_bytes!("asns.bin.br");

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct AsnMetadata {
    pub asn: u32,
    pub handle: Option<String>,
    pub description: Option<String>,
}

pub struct IpDatabase {
    ipv4_to_asn: PrefixMap<Ipv4Net, Arc<AsnMetadata>>,
    ipv6_to_asn: PrefixMap<Ipv6Net, Arc<AsnMetadata>>,
}

impl IpDatabase {
    pub fn from_static() -> IpDatabase {
        let asns_handle: JoinHandle<_> = spawn(|| {
            let asns_raw: BTreeMap<u32, AsnMetadata> =
                bincode::deserialize_from(brotli::Decompressor::new(ASN_METADATA_RAW_BR, 4096))
                    .expect("static ASN data is valid");

            let asns: HashMap<u32, Arc<AsnMetadata>> = asns_raw
                .into_iter()
                .map(|(asn, metadata)| (asn, Arc::new(metadata)))
                .collect();
            asns
        });
        let ipv4_to_asn_raw_handle: JoinHandle<BTreeMap<Ipv4Net, u32>> = spawn(|| {
            bincode::deserialize_from(brotli::Decompressor::new(IPV4_TO_ASN_RAW_BR, 4096))
                .expect("static IPv4 data is valid")
        });
        let ipv6_to_asn_raw_handle: JoinHandle<BTreeMap<Ipv6Net, u32>> = spawn(|| {
            bincode::deserialize_from(brotli::Decompressor::new(IPV6_TO_ASN_RAW_BR, 4096))
                .expect("static IPv6 data is valid")
        });

        let asns = asns_handle.join().expect("ASN parse thread succeeds");

        let (ipv4_to_asn, ipv6_to_asn) = thread::scope(|s| {
            let ipv4_to_asn_handle = s.spawn(|| {
                ipv4_to_asn_raw_handle
                    .join()
                    .expect("IPv4 parse thread succeeds")
                    .into_iter()
                    .map(|(net, asn)| (net, Arc::clone(asns.get(&asn).expect("ASN exist"))))
                    .collect()
            });
            let ipv6_to_asn_handle = s.spawn(|| {
                ipv6_to_asn_raw_handle
                    .join()
                    .expect("IPv6 parse thread succeeds")
                    .into_iter()
                    .map(|(net, asn)| (net, Arc::clone(asns.get(&asn).expect("ASN exist"))))
                    .collect()
            });

            (
                ipv4_to_asn_handle
                    .join()
                    .expect("IPv4 PrefixMap creation succeeds"),
                ipv6_to_asn_handle
                    .join()
                    .expect("IPv6 PrefixMap creation succeeds"),
            )
        });

        IpDatabase {
            ipv4_to_asn,
            ipv6_to_asn,
        }
    }

    pub fn lookup(&self, addr: impl Into<IpAddr>) -> Option<&Arc<AsnMetadata>> {
        match addr.into() {
            IpAddr::V4(v4) => self
                .ipv4_to_asn
                .get_lpm(&Ipv4Net::from(v4))
                .map(|(_net, addr)| addr),
            IpAddr::V6(v6) => self
                .ipv6_to_asn
                .get_lpm(&Ipv6Net::from(v6))
                .map(|(_net, addr)| addr),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        str::FromStr,
    };

    use pretty_assertions::assert_eq;

    use crate::{AsnMetadata, IpDatabase};

    #[test]
    fn exists() {
        let database = IpDatabase::from_static();

        assert_eq!(
            database
                .lookup(Ipv4Addr::new(8, 8, 8, 8))
                .expect("Google DNS exists")
                .as_ref(),
            &AsnMetadata {
                asn: 15169,
                handle: Some("GOOGLE".to_owned()),
                description: Some("Google LLC".to_owned()),
            }
        );

        assert_eq!(
            database
                .lookup(Ipv4Addr::new(9, 9, 9, 9))
                .expect("Quad 9 DNS exists")
                .as_ref(),
            &AsnMetadata {
                asn: 19281,
                handle: Some("QUAD9-1".to_owned()),
                description: Some("Quad9".to_owned()),
            }
        );

        assert_eq!(
            database
                .lookup(Ipv4Addr::new(1, 1, 1, 1))
                .expect("Cloudflare DNS exists")
                .as_ref(),
            &AsnMetadata {
                asn: 13335,
                handle: Some("CLOUDFLARENET".to_owned()),
                description: Some("Cloudflare, Inc.".to_owned()),
            }
        );

        assert_eq!(
            database
                .lookup(Ipv6Addr::from_str("2001:4860:4860::8888").expect("IPv6 parsed valid"))
                .expect("Google DNS exists")
                .as_ref(),
            &AsnMetadata {
                asn: 15169,
                handle: Some("GOOGLE".to_owned()),
                description: Some("Google LLC".to_owned()),
            }
        );

        assert_eq!(
            database
                .lookup(Ipv6Addr::from_str("2620:fe::9").expect("IPv6 parsed valid"))
                .expect("Quad 9 DNS exists")
                .as_ref(),
            &AsnMetadata {
                asn: 19281,
                handle: Some("QUAD9-1".to_owned()),
                description: Some("Quad9".to_owned()),
            }
        );

        assert_eq!(
            database
                .lookup(Ipv6Addr::from_str("2606:4700:4700::1111").expect("IPv6 parsed valid"))
                .expect("Cloudflare DNS exists")
                .as_ref(),
            &AsnMetadata {
                asn: 13335,
                handle: Some("CLOUDFLARENET".to_owned()),
                description: Some("Cloudflare, Inc.".to_owned()),
            }
        );
    }
}
