mod qdrant;
mod sql_lite;

pub trait HasMemoryId {
    fn memory_id(&self) -> String;
}

pub use qdrant::QdrantVectorStore;
pub use sql_lite::SqlLiteStore;
