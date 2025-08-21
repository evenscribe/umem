use crate::USER_ID_HEADER;
use anyhow::Result;
use axum::http::request::Parts;
use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        tool::{Extension, Parameters},
    },
    model::{ErrorData as McpError, *},
    schemars, tool, tool_handler, tool_router,
};
use tracing::debug;
use umem_controller::MemoryController;
use umem_proto_generated::generated;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMemoryRequest {
    pub text: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMemoriesByQueryRequest {
    pub query: String,
}

#[derive(Clone, Default)]
pub struct McpService {
    tool_router: ToolRouter<Self>,
}

fn extract_user_id(parts: Parts) -> String {
    parts
        .headers
        .get(USER_ID_HEADER)
        .expect("Missing user ID header")
        .to_str()
        .expect("Invalid user ID header value")
        .to_owned()
}

impl McpService {
    pub fn new() -> Self {
        debug!("Creating new McpService instance");
        let tool_router = Self::tool_router();
        let tools = tool_router.list_all();
        debug!(
            "Registered tools: {:?}",
            tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        Self { tool_router }
    }
}

#[tool_router]
impl McpService {
    #[tool(
        name = "add_memory",
        description = "Add a memory to umem persistence layer."
    )]
    async fn add_memory(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(AddMemoryRequest { text }): Parameters<AddMemoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        debug!("add_memory tool called with text: {}", text);
        let user_id = extract_user_id(parts);
        if text.is_empty() {
            return Err(McpError::new(
                ErrorCode::INVALID_REQUEST,
                "Memory content cannot be empty",
                None,
            ));
        }
        let _ = MemoryController::add_memory(generated::Memory {
            user_id,
            content: text,
            ..Default::default()
        })
        .await;

        Ok(CallToolResult::success(vec![]))
    }

    #[tool(
        name = "get_memory",
        description = "Get all memories for the current user."
    )]
    async fn get_memory(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let parameters = generated::GetMemoriesByUserIdParameters {
            user_id: extract_user_id(parts),
        };
        let memory_bulk: String = MemoryController::get_memories_by_user_id(parameters)
            .await
            .unwrap()
            .memories
            .iter()
            .map(|mem| serde_json::to_string(mem).unwrap())
            .collect::<Vec<String>>()
            .join("\n");
        Ok(CallToolResult::success(vec![Annotated::new(
            RawContent::Text(RawTextContent { text: memory_bulk }),
            None,
        )]))
    }

    #[tool(
        name = "get_memory_by_query",
        description = "Get memories for the current user related to a query."
    )]
    async fn get_memory_by_query(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(GetMemoriesByQueryRequest { query }): Parameters<GetMemoriesByQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        let parameters = generated::GetMemoriesByQueryParameters {
            user_id: extract_user_id(parts),
            query,
        };
        let memory_bulk: String = MemoryController::get_memories_by_query(parameters)
            .await
            .unwrap()
            .memories
            .iter()
            .map(|mem| serde_json::to_string(mem).unwrap())
            .collect::<Vec<String>>()
            .join("\n");
        Ok(CallToolResult::success(vec![Annotated::new(
            RawContent::Text(RawTextContent { text: memory_bulk }),
            None,
        )]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for McpService {
    fn get_info(&self) -> ServerInfo {
        debug!("McpService::get_info called");
        let tools = self.tool_router.list_all();
        debug!(
            "Available tools in get_info: {:?}",
            tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        ServerInfo {
            instructions: Some("An external Memory Persistence Layer for LLM and AI Agents".into()),
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}
