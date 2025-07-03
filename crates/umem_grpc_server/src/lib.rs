use anyhow::Result;
use tonic::transport::Server;

mod generated {
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

#[tonic::async_trait]
impl MemoryService for MemoryServiceImpl {
    async fn add_memory(
        &self,
        request: Request<Memory>,
    ) -> Result<Response<AddMemoryResponse>, Status> {
        let store = get_memory_store().await;
        let mut memory = request.into_inner();
        if memory.id.is_empty() {
            memory.id = Uuid::new_v4().to_string();
        }
        if memory.created_at.is_empty() {
            memory.created_at = Utc::now().to_rfc3339();
        }
        if memory.updated_at.is_empty() {
            memory.updated_at = memory.created_at.clone();
        }

        memory.status = MemoryStatus::Pending as i32;
        let memory_id = memory.id.clone();

        // TODO : figure out the files and shit

        let embedder = EmbeddingsGenerator::new("".to_string(), "".to_string(), "".to_string());

        if memory.content.is_empty() {
            return Err(Status::new(
                tonic::Code::InvalidArgument,
                "content is empty",
            ));
        }

        let mut vectors = embedder
            .generate_embeddings(vec![memory.content.clone()])
            .await
            .expect("embeddings has failed my guy");

        store
            .insert_embedding(
                Payload::try_from(json!(memory)).expect("Payload try from failed."),
                std::mem::take(&mut vectors[0]),
                memory.user_id.as_str(),
            )
            .await
            .expect("insert embeddings");

        let response = AddMemoryResponse {
            id: memory_id,
            memory: Some(memory),
        };

        Ok(Response::new(response))
    }

    async fn get_memory(
        &self,
        _request: Request<GetMemoryRequest>,
    ) -> Result<Response<Memory>, Status> {
        unimplemented!()
    }

    // TODO : fulfill these too
    async fn delete_memory(
        &self,
        _request: Request<DeleteMemoryRequest>,
    ) -> Result<Response<()>, Status> {
        unimplemented!()
    }

    async fn update_memory(&self, _request: Request<Memory>) -> Result<Response<()>, Status> {
        unimplemented!()
    }

    async fn list_memories(
        &self,
        _request: Request<ListMemoriesRequest>,
    ) -> Result<Response<ListMemoriesResponse>, Status> {
        unimplemented!()
    }
}
