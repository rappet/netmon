use std::net::Ipv4Addr;

use bytes::Bytes;
use chrono::Utc;
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    TokioAsyncResolver,
};
use ipnetwork::Ipv4Network;
use nm_service::{
    clap, clap::Parser, kafka_model::dns_scraping::DnsReverseRecord, tokio, Context, LibOpts,
    NMService, Result,
};

/// A rDNS ptr scraping tool.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Opts {
    #[arg(long, env)]
    pub network: Ipv4Network,

    #[command(flatten)]
    pub lib_opts: LibOpts,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let svc = NMService::init(&opts.lib_opts);

    let producer = svc.create_producer("rdns-ipv4").await?;

    let resolver = TokioAsyncResolver::tokio(ResolverConfig::quad9_tls(), ResolverOpts::default());

    for addr in opts.network.iter() {
        let addr_encoded = Bytes::from(addr.to_ipv6_mapped().octets().as_slice().to_owned());
        let timestamp = Utc::now()
            .timestamp()
            .try_into()
            .context("timestamp to large")?;

        let (ptr_records, no_record) = match resolver.reverse_lookup(addr.into()).await {
            Ok(res) => (res.into_iter().map(|ptr| ptr.to_string()).collect(), false),
            Err(err) if matches!(err.kind(), &ResolveErrorKind::NoRecordsFound { .. }) => {
                (vec![], true)
            }
            Err(err) => return Err(err.into()),
        };

        let record = DnsReverseRecord {
            timestamp_seconds: timestamp,
            ip_addr: addr_encoded,
            ptr_records,
            no_record,
        };

        let mut truncated_addr = addr.octets();
        truncated_addr[3] = 0;
        producer
            .send(&Ipv4Addr::from(truncated_addr).to_string(), &record)
            .await?;
    }

    producer.flush().await?;

    Ok(())
}
