use tonic::{Request, Response, Status};
use umem_controller::MemoryController;
use umem_proto_generated::{generated, MemoryBulk};

#[derive(Debug, Default)]
pub struct QdrantServiceImpl;

#[tonic::async_trait]
impl generated::memory_service_server::MemoryService for QdrantServiceImpl {
    async fn add_memory(
        &self,
        request: Request<generated::Memory>,
    ) -> Result<Response<()>, Status> {
        let memory = request.into_inner();

        if memory.content.is_empty() {
            return Err(Status::internal("Memory content is empty."));
        }

        MemoryController::add_memory(memory)
            .await
            .map_err(|e| Status::internal(format!("Failed to add memory: {}", e)))?;

        Ok(Response::new(()))
    }

    async fn add_memory_bulk(
        &self,
        request: Request<generated::MemoryBulk>,
    ) -> Result<Response<()>, Status> {
        let memory_bulk = request.into_inner();

        if memory_bulk.memories.is_empty() {
            return Err(Status::internal("Memories is empty."));
        }

        MemoryController::add_memory_bulk(memory_bulk)
            .await
            .map_err(|e| Status::internal(format!("Failed to add memory bulk: {}", e)))?;

        Ok(Response::new(()))
    }

    async fn update_memory(
        &self,
        request: Request<generated::UpdateMemoryParameters>,
    ) -> Result<Response<()>, Status> {
        let update_memory_parameters = request.into_inner();

        MemoryController::update_memory(update_memory_parameters)
            .await
            .map_err(|e| Status::internal(format!("Failed to update memory: {}", e)))?;

        Ok(Response::new(()))
    }

    async fn delete_memory(
        &self,
        request: Request<generated::DeleteMemoryParameters>,
    ) -> Result<Response<()>, Status> {
        let delete_memory_parameters = request.into_inner();

        MemoryController::delete_memory(delete_memory_parameters)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete memory: {}", e)))?;

        Ok(Response::new(()))
    }

    async fn get_memories_by_query(
        &self,
        request: Request<generated::GetMemoriesByQueryParameters>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        let get_memories_by_query_parameters = request.into_inner();

        let MemoryBulk { memories } =
            MemoryController::get_memories_by_query(get_memories_by_query_parameters)
                .await
                .map_err(|e| Status::internal(format!("Failed to get memories by query: {}", e)))?;

        Ok(Response::new(generated::MemoryBulk { memories }))
    }

    async fn get_memories_by_user_id(
        &self,
        request: Request<generated::GetMemoriesByUserIdParameters>,
    ) -> Result<Response<generated::MemoryBulk>, Status> {
        let get_memories_by_user_id = request.into_inner();

        let MemoryBulk { memories } =
            MemoryController::get_memories_by_user_id(get_memories_by_user_id)
                .await
                .map_err(|e| {
                    Status::internal(format!("Failed to get memories by user ID: {}", e))
                })?;

        Ok(Response::new(generated::MemoryBulk { memories }))
    }
}
