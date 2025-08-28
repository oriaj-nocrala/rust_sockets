use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &["proto/messages.proto", "proto/discovery.proto"],
        &["proto/"],
    )?;
    Ok(())
}