fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rustc-check-cfg=cfg(grpc_disabled)");

    #[cfg(feature = "grpc")]
    {
        use std::process::Command;
        let protoc_exists = Command::new("protoc").arg("--version").output().is_ok();

        if protoc_exists {
            tonic_build::compile_protos("proto/pointer.proto")?;
            tonic_build::compile_protos("proto/register.proto")?;
            tonic_build::compile_protos("proto/chunk.proto")?;
            tonic_build::compile_protos("proto/graph.proto")?;
            tonic_build::compile_protos("proto/command.proto")?;
            tonic_build::compile_protos("proto/pnr.proto")?;
            tonic_build::compile_protos("proto/public_data.proto")?;
            tonic_build::compile_protos("proto/public_archive.proto")?;
            tonic_build::compile_protos("proto/tarchive.proto")?;
            tonic_build::compile_protos("proto/archive.proto")?;
            tonic_build::compile_protos("proto/private_scratchpad.proto")?;
            tonic_build::compile_protos("proto/public_scratchpad.proto")?;
            tonic_build::compile_protos("proto/scratchpad.proto")?;
            tonic_build::compile_protos("proto/resolver.proto")?;
            tonic_build::compile_protos("proto/key_value.proto")?;
        } else {
            println!("cargo:warning=protoc not found, disabling gRPC support");
            println!("cargo:rustc-cfg=grpc_disabled");
        }
    }
    #[cfg(not(feature = "grpc"))]
    {
        println!("cargo:rustc-cfg=grpc_disabled");
    }

    Ok(())
}
