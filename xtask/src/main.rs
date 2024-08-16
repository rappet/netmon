use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use ipnet::{Ipv4Net, Ipv6Net};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Converts data from https://github.com/ipverse/asn-ip to a more compact format
pub(crate) struct Opts {
    #[arg(long, env)]
    pub(crate) input_archive: PathBuf,

    #[arg(long, env, default_value = "lib/ip-lookup/src/ipv4_to_asn.bin.br")]
    pub(crate) ipv4_output: PathBuf,

    #[arg(long, env, default_value = "lib/ip-lookup/src/ipv6_to_asn.bin.br")]
    pub(crate) ipv6_output: PathBuf,

    #[arg(long, env, default_value = "lib/ip-lookup/src/asns.bin.br")]
    pub(crate) asns_output: PathBuf,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct InputFile {
    pub asn: u32,
    pub handle: Option<String>,
    pub description: Option<String>,
    pub subnets: Subnets,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Subnets {
    pub ipv4: Vec<Ipv4Net>,
    pub ipv6: Vec<Ipv6Net>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct AsnMetadata {
    pub asn: u32,
    pub handle: Option<String>,
    pub description: Option<String>,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    let mut archive = ZipArchive::new(BufReader::new(File::open(&opts.input_archive)?))?;

    let mut ipv4_to_asn: BTreeMap<Ipv4Net, u32> = BTreeMap::new();
    let mut ipv6_to_asn: BTreeMap<Ipv6Net, u32> = BTreeMap::new();
    let mut asn_to_metadata: BTreeMap<u32, AsnMetadata> = BTreeMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if !file.name().ends_with(".json") {
            continue;
        }

        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        let parsed: InputFile = simd_json::from_slice(&mut content)?;

        for ipv4 in parsed.subnets.ipv4 {
            ipv4_to_asn.insert(ipv4, parsed.asn);
        }

        for ipv6 in parsed.subnets.ipv6 {
            ipv6_to_asn.insert(ipv6, parsed.asn);
        }

        asn_to_metadata.insert(
            parsed.asn,
            AsnMetadata {
                asn: parsed.asn,
                handle: parsed.handle,
                description: parsed.description,
            },
        );
    }

    eprintln!("writing ipv4->asn...");
    let mut ipv4_writer =
        brotli::CompressorWriter::new(File::create(opts.ipv4_output)?, 4096, 11, 22);
    bincode::serialize_into(&mut ipv4_writer, &ipv4_to_asn)?;
    ipv4_writer.flush()?;
    drop(ipv4_writer);

    eprintln!("writing ipv6->asn...");
    let mut ipv6_writer =
        brotli::CompressorWriter::new(File::create(opts.ipv6_output)?, 4096, 11, 22);
    bincode::serialize_into(&mut ipv6_writer, &ipv6_to_asn)?;
    ipv6_writer.flush()?;
    drop(ipv6_writer);

    eprintln!("writing asn metadata...");
    let mut asns_writer =
        brotli::CompressorWriter::new(File::create(opts.asns_output)?, 4096, 11, 22);
    bincode::serialize_into(&mut asns_writer, &asn_to_metadata)?;
    asns_writer.flush()?;
    drop(asns_writer);

    Ok(())
}
