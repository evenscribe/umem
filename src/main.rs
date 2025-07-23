use anyhow::Result;
use dotenv::dotenv;
use umem_grpc_server::MemoryServiceGrpc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let _ = MemoryServiceGrpc::run_server("[::1]:50051").await;
    Ok(())
}
