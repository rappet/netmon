pub(crate) mod ip_addr {
    use std::{
        net::{IpAddr, Ipv6Addr},
        str::FromStr,
    };

    use bytes::Bytes;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut octets = [0u8; 16];
        if bytes.len() != 16 {
            return Err(serde::ser::Error::custom(format!(
                "binary encoded IP address has the wrong length {}, length has to be 16",
                bytes.len()
            )));
        }
        octets.copy_from_slice(&bytes);
        let addr = Ipv6Addr::from(octets);
        let formatted = if let Some(v4) = addr.to_ipv4() {
            v4.to_string()
        } else {
            addr.to_string()
        };
        serializer.serialize_str(&formatted)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ip_addr = IpAddr::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(match ip_addr {
            IpAddr::V4(v4) => Bytes::from(v4.to_ipv6_mapped().octets().to_vec()),
            IpAddr::V6(v6) => Bytes::from(v6.octets().to_vec()),
        })
    }
}

pub(crate) mod timestamp_seconds {
    use chrono::DateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(timestamp: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = DateTime::from_timestamp((*timestamp).into(), 0)
            .unwrap()
            .to_rfc3339();
        serializer.serialize_str(&formatted)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let datetime = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
        Ok(u32::try_from(datetime.timestamp()).map_err(|_| serde::de::Error::custom("datetime is out of range"))?)
    }
}
