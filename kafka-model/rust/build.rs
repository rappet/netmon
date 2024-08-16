use std::io::Result;

use prost_build::Config;

fn main() -> Result<()> {
    // We generate to src because IntelliJ does not regenerate completions et. al. if using OUT_DIR
    Config::default()
        .bytes(["."])
        .out_dir("src/generated")
        .compile_protos(&["../proto/packet-sample.proto"], &["../proto/"])?;
    Ok(())
}
