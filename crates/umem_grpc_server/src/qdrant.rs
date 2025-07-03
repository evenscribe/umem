use crate::generated;
use anyhow::Result;
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};
use umem_vector::MemoryStore;

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

#[derive(Debug, Default)]
pub struct QdrantServiceImpl;

#[tonic::async_trait]
impl generated::memory_service_server::MemoryService for QdrantServiceImpl {
    async fn add_memory(
        &self,
        request: Request<generated::Memory>,
    ) -> Result<Response<generated::Memory>, Status> {
        let memory_store = get_memory_store().await;

        memory_store.insert_embedding(payload, vectors, user_id);

        todo!()
    }

    async fn add_memory_bulk(
        &self,
        _request: Request<generated::MemoryBulk>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        todo!()
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
