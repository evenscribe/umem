use chrono::Utc;
use memory::memory_service_server::{MemoryService, MemoryServiceServer};
pub use memory::*;
use serde_json::json;
use tokio::sync::OnceCell;
use tonic::{transport::Server, Request, Response, Status};
use umem_embeddings::EmbeddingsGenerator;
use umem_vector::{MemoryStore, Payload};
use uuid::Uuid;

pub mod memory {
    tonic::include_proto!("memory");
}

static MEMORY_STORE: OnceCell<MemoryStore> = OnceCell::const_new();

const URL: &str = "http://localhost:6334";
const KEY: &str = "test";
const COLLECTION_NAME: &str = "coll";

async fn get_memory_store() -> &'static MemoryStore {
    MEMORY_STORE
        .get_or_init(|| async {
            MemoryStore::new(URL, KEY, COLLECTION_NAME)
                .await
                .expect("qdrant client failed to intialize")
        })
        .await
}

#[derive(Debug, Default)]
pub struct MemoryServiceImpl;

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

        let embedder = EmbeddingsGenerator::new(
            "@cf/baai/bge-m3".to_string(),
            "dffeced6f514ef3472c7ed13fada97b2".to_string(),
            "GbJllQYGLLaQ42DmsxJnebOgJB-FzMFoT_9OrtDN".to_string(),
        );

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

pub async fn run_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = addr.parse()?;

    println!("Memory gRPC Server listening on {}", addr);

    Server::builder()
        .add_service(MemoryServiceServer::new(MemoryServiceImpl))
        .serve(addr)
        .await?;

    Ok(())
}
