use anyhow::Result;
use tonic::transport::Server;

mod qdrant;

pub(crate) mod generated {
    tonic::include_proto!("memory");
}

pub struct MemoryServiceGrpc;

impl MemoryServiceGrpc {
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
