use anyhow::Result;
use rmcp::{
    ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ErrorData as McpError, *},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use umem_controller::MemoryController;
use umem_proto_generated::generated;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMemoryRequst {
    pub memory: generated::Memory,
}

#[derive(Clone)]
pub struct McpService {
    tool_router: ToolRouter<Self>,
}

impl McpService {
    pub fn new() -> Self {
        Self {
            tool_router: ToolRouter::new(),
        }
    }
}

#[tool_router]
impl McpService {
    #[tool(
        name = "add_memory",
        description = "Add a memory to umem persistence layer"
    )]
    async fn add_memory(
        &self,
        Parameters(AddMemoryRequst { memory }): Parameters<AddMemoryRequst>,
    ) -> Result<CallToolResult, McpError> {
        if memory.content.is_empty() {
            return Err(McpError::new(
                ErrorCode::INVALID_REQUEST,
                "Memory content cannot be empty".into(),
                None,
            ));
        }
        MemoryController::add_memory(memory);
        Ok(CallToolResult::success("Memory added successfully".into()))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for McpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("An external Memory Persistence Layer for LLM and AI Agents".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl McpService {
    pub async fn run_server(_addr: &str) -> Result<()> {
        let service = McpService::new().serve(stdio()).await.inspect_err(|e| {
            println!("Error starting server: {}", e);
        })?;
        service.waiting().await?;

        Ok(())
    }
}
