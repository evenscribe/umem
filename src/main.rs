use anyhow::Result;
use dotenv::dotenv;
use umem_grpc_server::MemoryServiceGrpc;
use umem_search::ProjectDirs;
use umem_search::TraceIndex;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    TraceIndex::create_index(ProjectDirs::get_trace_index_path()?)?;
    let _ = MemoryServiceGrpc::run_server("[::1]:50051").await;
    Ok(())
}
