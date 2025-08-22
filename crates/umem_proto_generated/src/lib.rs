mod memory;

use rmcp::schemars;
use umem_utils::{PointId, QdrantIdentifiable};

impl QdrantIdentifiable for generated::Memory {
    fn get_id(&self) -> impl Into<PointId> {
        self.memory_id.clone()
    }
}

pub mod generated {
    tonic::include_proto!("memory");
}
