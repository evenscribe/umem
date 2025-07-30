use anyhow::Result;
use dotenv::dotenv;
use umem_grpc_server::MemoryServiceGrpc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let mcp_handle = tokio::spawn(async move { umem_mcp::run_server().await });
    let grpc_handle =
        tokio::spawn(async move { MemoryServiceGrpc::run_server("[::1]:50051").await });

    let _ = tokio::try_join!(mcp_handle, grpc_handle)?;

    Ok(())
}
