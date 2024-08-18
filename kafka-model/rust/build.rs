use std::io::Result;

use prost_build::Config;

fn main() -> Result<()> {
    // We generate to src because IntelliJ does not regenerate completions et. al. if using OUT_DIR
    Config::default()
        .bytes(["."])
        .out_dir("src/generated")
        .message_attribute(".", "#[derive(::serde::Serialize, ::serde::Deserialize)]")
        .compile_protos(
            &[
                "../proto/packet-sample.proto",
                "../proto/dns-scraping.proto",
            ],
            &["../proto/"],
        )?;
    Ok(())
}
