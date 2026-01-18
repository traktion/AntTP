fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/pointer.proto")?;
    tonic_build::compile_protos("proto/chunk.proto")?;
    Ok(())
}
