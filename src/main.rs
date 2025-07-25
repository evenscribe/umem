use anyhow::Result;
use dotenv::dotenv;
use umem_controller::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _ = get_memory_store().await;
    // let _ = MemoryServiceGrpc::run_server("[::1]:50051").await;
    umem_mcp::McpService::run_server("").await?;
    Ok(())
}
