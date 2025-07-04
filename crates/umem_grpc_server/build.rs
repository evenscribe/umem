fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        // .out_dir("src")
        .type_attribute("Memory", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["proto/memory.proto"], &["proto"])?;
    Ok(())
}
