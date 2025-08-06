use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ErrorData as McpError, *},
    schemars, tool, tool_handler, tool_router,
};
use umem_controller::MemoryController;
use umem_proto_generated::generated;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMemoryRequst {
    pub text: String,
}

#[derive(Clone, Default)]
pub struct McpService {
    tool_router: ToolRouter<Self>,
}

impl McpService {
    pub fn new() -> Self {
        println!("Creating new McpService instance");
        let tool_router = Self::tool_router();
        let tools = tool_router.list_all();
        println!(
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
        description = "Add a memory to umem persistence layer"
    )]
    async fn add_memory(
        &self,
        Parameters(AddMemoryRequst { text }): Parameters<AddMemoryRequst>,
    ) -> Result<CallToolResult, McpError> {
        println!("add_memory tool called with text: {}", text);
        if text.is_empty() {
            return Err(McpError::new(
                ErrorCode::INVALID_REQUEST,
                "Memory content cannot be empty",
                None,
            ));
        }

        let _ = MemoryController::add_memory(generated::Memory {
            user_id: "12133".into(),
            content: text,
            ..Default::default()
        })
        .await;

        Ok(CallToolResult::success(vec![]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for McpService {
    fn get_info(&self) -> ServerInfo {
        println!("McpService::get_info called");
        let tools = self.tool_router.list_all();
        println!(
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
