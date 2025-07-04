use crate::generated;
use futures::future::join_all;
use lazy_static::lazy_static;
use serde_json::json;
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};
use umem_embeddings::Embedder;
use umem_vector::{MemoryStore, Payload};

const URL: &str = "";
const KEY: &str = "";
const COLLECTION_NAME: &str = "";

static MEMORY_STORE: OnceCell<MemoryStore> = OnceCell::const_new();
async fn get_memory_store() -> &'static MemoryStore {
    MEMORY_STORE
        .get_or_init(|| async {
            MemoryStore::new(URL, KEY, COLLECTION_NAME)
                .await
                .expect("qdrant client failed to intialize")
        })
        .await
}

lazy_static! {
    static ref CFEmbeder: umem_embeddings::CfBaaiBgeM3Embeder =
        umem_embeddings::CfBaaiBgeM3Embeder::new(
            std::env::var("CLOUDFLARE_ACCOUNT_ID").expect("CLOUDFLARE_ACCOUNT_ID not set"),
            std::env::var("CLOUDFLARE_API_TOKEN").expect("CLOUDFLARE_API_TOKEN not set"),
        );
}

#[derive(Debug, Default)]
pub struct QdrantServiceImpl;

#[tonic::async_trait]
impl generated::memory_service_server::MemoryService for QdrantServiceImpl {
    async fn add_memory(
        &self,
        request: Request<generated::Memory>,
    ) -> Result<Response<generated::Memory>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        let vectors = CFEmbeder
            .generate_embedding(request.content.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to generate embedding: {}", e)))?;

        let payload = Payload::try_from(json!(request)).map_err(|e| {
            Status::invalid_argument(format!("Failed to convert Memory to Payload: {}", e))
        })?;

        memory_store
            .insert_embedding(payload, vectors, request.user_id.as_str())
            .await
            .map_err(|e| Status::internal(format!("Failed to upsert memory: {}", e)))?;

        Ok(Response::new(request))
    }

    async fn add_memory_bulk(
        &self,
        request: Request<generated::MemoryBulk>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        let vectors: Vec<_> = request
            .memories
            .iter()
            .map(
                async |memory| match CFEmbeder.generate_embedding(memory.content.clone()).await {
                    Ok(vectors) => Some(vectors),
                    Err(e) => None,
                },
            )
            .collect();

        let vectors: Vec<Option<Vec<f32>>> = join_all(vectors).await;

        let payload = Payload::try_from(json!(request)).map_err(|e| {
            Status::invalid_argument(format!("Failed to convert Memory to Payload: {}", e))
        })?;

        memory_store
            .insert_embedding(payload, vectors, request.user_id.as_str())
            .await
            .map_err(|e| Status::internal(format!("Failed to upsert memory: {}", e)))?;

        Ok(Response::new(request))
    }

    async fn modify_memory(
        &self,
        _request: Request<generated::ModifyMemoryParameters>,
    ) -> Result<Response<generated::Memory>, Status> {
        todo!()
    }

    async fn delete_memory(
        &self,
        _request: Request<generated::DeleteMemoryParameters>,
    ) -> Result<Response<()>, Status> {
        todo!()
    }

    /// Qdrant Queries
    async fn get_memory_by_query(
        &self,
        _request: Request<generated::GetMemoriesByQueryParameters>,
    ) -> Result<Response<generated::Memory>, Status> {
        todo!()
    }

    async fn get_memory_by_user_id(
        &self,
        _request: Request<generated::GetMemoriesByUserIdParameters>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        todo!()
    }
}
