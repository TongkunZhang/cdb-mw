fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/storageservice.proto")?;
    tonic_build::compile_protos("proto/filetransfer.proto")?;
    Ok(())
}
