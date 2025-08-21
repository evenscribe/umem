use anyhow::Result;
use tonic::transport::Server;
use tracing::info;
use umem_proto_generated::generated;

mod qdrant;

pub struct MemoryServiceGrpc;

impl MemoryServiceGrpc {
    pub async fn run_server(addr: &str) -> Result<()> {
        let addr = addr.parse()?;
        info!("Memory gRPC Server listening on {}", addr);
        Server::builder()
            .add_service(generated::memory_service_server::MemoryServiceServer::new(
                qdrant::QdrantServiceImpl,
            ))
            .serve(addr)
            .await?;
        Ok(())
    }
}
