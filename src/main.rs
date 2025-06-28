use anyhow::Result;
use umem_grpc_server::run_server;
use umem_search::ProjectDirs;
use umem_search::TraceIndex;

#[tokio::main]
async fn main() -> Result<()> {
    TraceIndex::create_index(ProjectDirs::get_trace_index_path()?)?;
    let _ = run_server("[::1]:50051").await;
    Ok(())
}
