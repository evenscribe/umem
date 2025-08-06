extern crate rmcp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        // .out_dir("./src/")
        .type_attribute("Memory", "use crate::schemars;")
        .type_attribute(
            "Memory",
            "#[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "UpdateMemoryParameters",
            "#[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(&["proto/memory.proto"], &["proto"])?;
    Ok(())
}
