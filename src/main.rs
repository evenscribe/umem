use anyhow::Result;
use dotenv::dotenv;
use umem_grpc_server::MemoryServiceGrpc;

mod tracing;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _guard = tracing::init_tracing()?;

    let mcp_handle = tokio::spawn(async move { umem_mcp::run_server().await });
    let grpc_handle =
        tokio::spawn(async move { MemoryServiceGrpc::run_server("0.0.0.0:5051").await });

    let _ = tokio::try_join!(mcp_handle, grpc_handle)?;

    Ok(())
}
