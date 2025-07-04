use anyhow::Result;
use tonic::transport::Server;

pub(crate) mod generated {
    tonic::include_proto!("memory");
}
mod qdrant;
pub struct MemoryService;

impl MemoryService {
    pub async fn run_server(addr: &str) -> Result<()> {
        let addr = addr.parse()?;

        println!("Memory gRPC Server listening on {}", addr);

        Server::builder()
            .add_service(generated::memory_service_server::MemoryServiceServer::new(
                qdrant::QdrantServiceImpl,
            ))
            .serve(addr)
            .await?;

        Ok(())
    }
}
