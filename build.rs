fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/pointer.proto")?;
    tonic_build::compile_protos("proto/register.proto")?;
    tonic_build::compile_protos("proto/chunk.proto")?;
    tonic_build::compile_protos("proto/graph.proto")?;
    tonic_build::compile_protos("proto/command.proto")?;
    tonic_build::compile_protos("proto/pnr.proto")?;
    tonic_build::compile_protos("proto/public_data.proto")?;
    tonic_build::compile_protos("proto/public_archive.proto")?;
    tonic_build::compile_protos("proto/tarchive.proto")?;
    tonic_build::compile_protos("proto/private_scratchpad.proto")?;
    tonic_build::compile_protos("proto/public_scratchpad.proto")?;
    tonic_build::compile_protos("proto/scratchpad.proto")?;
    Ok(())
}
