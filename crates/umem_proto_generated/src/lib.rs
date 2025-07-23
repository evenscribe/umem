mod memory;

use rmcp::schemars;
use umem_db::HasMemoryId;

pub mod generated {
    tonic::include_proto!("memory");
}

impl HasMemoryId for generated::Memory {
    fn memory_id(&self) -> String {
        self.memory_id.clone()
    }
}
