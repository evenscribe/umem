use crate::generated;
use lazy_static::lazy_static;
use serde_json::json;
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};
use umem_embeddings::Embedder;
use umem_vector::QdrantVectorStore;

static MEMORY_STORE: OnceCell<QdrantVectorStore> = OnceCell::const_new();

async fn get_memory_store() -> &'static QdrantVectorStore {
    MEMORY_STORE
        .get_or_init(|| async {
            QdrantVectorStore::new(
                &std::env::var("QDRANT_URL").expect("QDRANT_URL not set"),
                &std::env::var("QDRANT_KEY").expect("QDRANT_KEY not set"),
                &std::env::var("QDRANT_COLLECTION_NAME").expect("QDRANT_COLLECTION_NAME not set"),
            )
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
    ) -> Result<Response<()>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();
        let vectors = CFEmbeder
            .generate_embedding(request.content.as_str())
            .await
            .map_err(|e| Status::internal(format!("Failed to generate embedding: {}", e)))?;
        memory_store
            .insert_embedding(request, vectors)
            .await
            .map_err(|e| Status::internal(format!("Failed to upsert memory: {}", e)))?;
        Ok(Response::new(()))
    }

    async fn add_memory_bulk(
        &self,
        request: Request<generated::MemoryBulk>,
    ) -> Result<Response<()>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        if request.memories.is_empty() {
            return Err(Status::internal("Memories is empty."));
        }

        let texts = request
            .memories
            .iter()
            .map(|memory| memory.content.as_str())
            .collect();

        let vectors: Vec<Vec<f32>> = CFEmbeder
            .generate_embeddings_bulk(texts)
            .await
            .map_err(|e| Status::internal(format!("Failed to generate embedding: {}", e)))?;

        memory_store
            .insert_embeddings_bulk(std::iter::zip(request.memories, vectors).collect::<Vec<_>>())
            .await
            .map_err(|e| Status::internal(format!("Failed to upsert memory: {}", e)))?;

        Ok(Response::new(()))
    }

    async fn update_memory(
        &self,
        request: Request<generated::UpdateMemoryParameters>,
    ) -> Result<Response<()>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        let vectors = CFEmbeder
            .generate_embedding(&request.content.as_str())
            .await
            .map_err(|e| Status::internal(format!("Failed to generate embedding: {}", e)))?;

        memory_store
            .update_point(&request.memory_id.clone(), Some(vectors), Some(request))
            .await
            .map_err(|e| Status::internal(format!("Failed to modify_memory : {}", e)))?;

        Ok(Response::new(()))
    }

    async fn delete_memory(
        &self,
        request: Request<generated::DeleteMemoryParameters>,
    ) -> Result<Response<()>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        memory_store
            .delete_point(&request.memory_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete memory: {}", e)))?;

        Ok(Response::new(()))
    }

    /// Qdrant Queries
    async fn get_memories_by_query(
        &self,
        request: Request<generated::GetMemoriesByQueryParameters>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        let vector = CFEmbeder
            .generate_embedding(&request.query)
            .await
            .map_err(|e| Status::internal(format!("Failed to generate_embedding : {}", e)))?;

        let search_response = memory_store
            .search_with_vector(vector, Some(10), &request.user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to search_with_vector : {}", e)))?;

        Ok(Response::new(generated::MemoryBulk {
            memories: search_response
                .result
                .into_iter()
                .map(|scored_point| {
                    serde_json::from_value(json!(scored_point.payload))
                        .expect("Payload to Memory parse failed.")
                })
                .collect::<Vec<_>>(),
        }))
    }

    async fn get_memories_by_user_id(
        &self,
        request: Request<generated::GetMemoriesByUserIdParameters>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        let memory_store = get_memory_store().await;
        let request = request.into_inner();

        let search_response = memory_store
            .search_with_payload(vec![("user_id".to_string(), request.user_id)], None)
            .await
            .map_err(|e| Status::internal(format!("Failed to search_with_vector : {}", e)))?;

        Ok(Response::new(generated::MemoryBulk {
            memories: search_response
                .result
                .into_iter()
                .map(|scored_point| {
                    serde_json::from_value(json!(scored_point.payload))
                        .expect("Payload to Memory parse failed.")
                })
                .collect::<Vec<_>>(),
        }))
    }
}
