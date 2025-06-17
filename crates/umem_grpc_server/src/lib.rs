use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

pub mod memory {
    tonic::include_proto!("memory");
}

use memory::memory_service_server::{MemoryService, MemoryServiceServer};
pub use memory::*;

#[derive(Debug, Default)]
pub struct MemoryServiceImpl {
    memories: Arc<Mutex<HashMap<String, Memory>>>,
}

impl MemoryServiceImpl {
    pub fn new() -> Self {
        Self {
            memories: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl MemoryService for MemoryServiceImpl {
    async fn add_memory(
        &self,
        request: Request<Memory>,
    ) -> Result<Response<AddMemoryResponse>, Status> {
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

        if memory.status == MemoryStatus::Pending as i32 {
            memory.status = MemoryStatus::Pending as i32;
        }

        let memory_id = memory.id.clone();
        let memory_clone = memory.clone();

        let mut memories = self.memories.lock().unwrap();
        memories.insert(memory_id.clone(), memory);

        let response = AddMemoryResponse {
            id: memory_id,
            memory: Some(memory_clone),
        };

        Ok(Response::new(response))
    }

    async fn get_memory(
        &self,
        request: Request<GetMemoryRequest>,
    ) -> Result<Response<Memory>, Status> {
        let req = request.into_inner();
        let memories = self.memories.lock().unwrap();

        match memories.get(&req.id) {
            Some(memory) => Ok(Response::new(memory.clone())),
            None => Err(Status::not_found(format!(
                "Memory with id {} not found",
                req.id
            ))),
        }
    }

    async fn delete_memory(
        &self,
        request: Request<DeleteMemoryRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let mut memories = self.memories.lock().unwrap();

        match memories.remove(&req.id) {
            Some(_) => Ok(Response::new(())),
            None => Err(Status::not_found(format!(
                "Memory with id {} not found",
                req.id
            ))),
        }
    }

    async fn update_memory(&self, request: Request<Memory>) -> Result<Response<()>, Status> {
        let mut memory = request.into_inner();
        let mut memories = self.memories.lock().unwrap();

        if !memories.contains_key(&memory.id) {
            return Err(Status::not_found(format!(
                "Memory with id {} not found",
                memory.id
            )));
        }

        memory.updated_at = Utc::now().to_rfc3339();
        memories.insert(memory.id.clone(), memory);

        Ok(Response::new(()))
    }

    async fn list_memories(
        &self,
        request: Request<ListMemoriesRequest>,
    ) -> Result<Response<ListMemoriesResponse>, Status> {
        let req = request.into_inner();
        let memories = self.memories.lock().unwrap();

        let mut filtered_memories: Vec<Memory> = memories.values().cloned().collect();

        if !req.tags.is_empty() {
            filtered_memories.retain(|memory| req.tags.iter().any(|tag| memory.tags.contains(tag)));
        }

        if let Some(sort_by) = req.sort_by {
            match sort_by.field.as_str() {
                "created_at" => {
                    if sort_by.order == "desc" {
                        filtered_memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                    } else {
                        filtered_memories.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                    }
                }
                "title" => {
                    if sort_by.order == "desc" {
                        filtered_memories.sort_by(|a, b| b.title.cmp(&a.title));
                    } else {
                        filtered_memories.sort_by(|a, b| a.title.cmp(&b.title));
                    }
                }
                _ => {}
            }
        }

        let total_count = filtered_memories.len() as i32;
        let page = if req.page > 0 { req.page } else { 1 };
        let limit = if req.limit > 0 { req.limit } else { 10 };

        let start_index = ((page - 1) * limit) as usize;
        let end_index = (start_index + limit as usize).min(filtered_memories.len());

        let paginated_memories = if start_index < filtered_memories.len() {
            filtered_memories[start_index..end_index].to_vec()
        } else {
            vec![]
        };

        let response = ListMemoriesResponse {
            memories: paginated_memories,
            total_count,
            page,
            page_size: limit,
        };

        Ok(Response::new(response))
    }
}

pub async fn run_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = addr.parse()?;
    let memory_service = MemoryServiceImpl::new();

    println!("Memory gRPC Server listening on {}", addr);

    Server::builder()
        .add_service(MemoryServiceServer::new(memory_service))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_add_memory() {
        let service = MemoryServiceImpl::new();

        let memory = Memory {
            id: String::new(),
            title: "Test Memory".to_string(),
            summary: "A test memory".to_string(),
            custom_id: String::new(),
            kind: MemoryKind::PlainText as i32,
            content: "This is test content".to_string(),
            tags: vec!["test".to_string()],
            metadata: "{}".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            status: MemoryStatus::Pending as i32,
            raw_content: String::new(),
            user_id: "user123".to_string(),
            token: String::new(),
        };

        let request = Request::new(memory);
        let response = service.add_memory(request).await.unwrap();
        let add_response = response.into_inner();

        assert!(!add_response.id.is_empty());
        assert!(add_response.memory.is_some());

        let returned_memory = add_response.memory.unwrap();
        assert_eq!(returned_memory.title, "Test Memory");
        assert!(!returned_memory.created_at.is_empty());
    }

    #[tokio::test]
    async fn test_get_memory() {
        let service = MemoryServiceImpl::new();

        let memory = Memory {
            id: String::new(),
            title: "Test Memory".to_string(),
            summary: "A test memory".to_string(),
            custom_id: String::new(),
            kind: MemoryKind::PlainText as i32,
            content: "This is test content".to_string(),
            tags: vec!["test".to_string()],
            metadata: "{}".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            status: MemoryStatus::Pending as i32,
            raw_content: String::new(),
            user_id: "user123".to_string(),
            token: String::new(),
        };

        let add_request = Request::new(memory);
        let add_response = service.add_memory(add_request).await.unwrap();
        let memory_id = add_response.into_inner().id;

        let get_request = Request::new(GetMemoryRequest { id: memory_id });
        let get_response = service.get_memory(get_request).await.unwrap();
        let retrieved_memory = get_response.into_inner();

        assert_eq!(retrieved_memory.title, "Test Memory");
    }
}
