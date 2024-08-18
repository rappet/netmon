use std::io::Result;

use prost_build::Config;

fn main() -> Result<()> {
    // We generate to src because IntelliJ does not regenerate completions et. al. if using OUT_DIR
    Config::default()
        .bytes(["."])
        .out_dir("src/generated")
        .message_attribute(".", "#[derive(::serde::Serialize, ::serde::Deserialize)]")
        .field_attribute("ip_addr", r#"#[serde(with = "crate::serde::ip_addr")]"#)
        .field_attribute("timestamp_seconds", r#"#[serde(with = "crate::serde::timestamp_seconds", rename = "timestamp")]"#)
        .compile_protos(
            &[
                "../proto/packet-sample.proto",
                "../proto/dns-scraping.proto",
            ],
            &["../proto/"],
        )?;
    Ok(())
}
