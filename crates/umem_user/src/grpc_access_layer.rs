use anyhow::Result;
use umem_proto_generated::generated;

type MemoryId = String;
pub struct GrpcAccessLayer;

impl GrpcAccessLayer {
    pub async fn add_memory(memory: generated::Memory) -> Result<MemoryId> {}

    // pub async fn add_memory_bulk(memory_bulk: generated::MemoryBulk) -> Result<()> {}

    // pub async fn update_memory(
    //     update_memory_parameters: generated::UpdateMemoryParameters,
    // ) -> Result<()> {
    // }

    // pub async fn delete_memory(
    //     delete_memory_parameters: generated::DeleteMemoryParameters,
    // ) -> Result<()> {
    // }

    // /// Qdrant Queries
    // pub async fn get_memories_by_query(
    //     get_memories_by_query_parameters: generated::GetMemoriesByQueryParameters,
    // ) -> Result<generated::MemoryBulk> {
    // }

    // pub async fn get_memories_by_user_id(
    //     get_memories_by_user_id_parameters: generated::GetMemoriesByUserIdParameters,
    // ) -> Result<generated::MemoryBulk> {
    // }
}
