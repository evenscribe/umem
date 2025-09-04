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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateMememoryRequest {
    pub memory_id: String,
    pub content: String,
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
        description = "Add a memory to umem persistence layer. This tool MUST be used frequently to store important information for future conversations. The AI MUST proactively save: 1) All user preferences and personal details, 2) Any facts that would be tedious for users to repeat, 3) Key insights discovered during conversation, 4) Important context about projects or relationships. Every substantive exchange should trigger memory storage - this is NOT optional but essential functionality. Failing to save memories will severely degrade user experience as the system will forget critical context between sessions. Save concise, structured memories frequently throughout the conversation, even for seemingly casual but potentially useful information."
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

        let memory = MemoryController::add_memory(generated::Memory {
            user_id,
            content: text,
            ..Default::default()
        })
        .await
        .unwrap();

        Ok(CallToolResult::success(vec![Annotated::new(
            RawContent::Text(RawTextContent {
                text: serde_json::to_string(&memory).unwrap(),
            }),
            None,
        )]))
    }

    #[tool(
        name = "get_memory",
        description = "Get all memories for the current user. Retrieves the user's persistent memory store containing important context, preferences, and historical interactions. This tool should be called at the beginning of conversations to load relevant contextual information and provide personalized responses based on past interactions. After using this information, remember to save new important details using add_memory."
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
        description = "Get memories for the current user related to a query. This tool enables targeted retrieval of specific memories from the persistence layer using semantic search capabilities. WHEN TO USE: (1) When responding to questions that may benefit from past context, (2) Before generating responses that should consider historical preferences or interactions, (3) When references to previous conversations are made, or (4) When topic-specific context would improve response quality. IMPLEMENTATION: The query parameter accepts natural language or keywords—umem automatically performs hybrid semantic and keyword matching to retrieve the most relevant memories. BEST PRACTICE: Use focused, specific queries rather than generic ones for better results. After retrieving memories, consider saving new insights with add_memory to maintain an up-to-date persistence layer."
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

    #[tool(
        name = "update_memory",
        description = "Update an existing memory in the umem persistence layer. This tool should be used to modify, correct, or enhance existing memories rather than creating duplicates. WHEN TO USE: (1) When user preferences or details change and need to be updated in existing memories, (2) When previously stored information becomes outdated or incorrect, (3) When additional context or clarification needs to be added to existing memories, (4) When consolidating or refining memories to avoid redundancy, (5) When correcting errors or inaccuracies in stored memories. CRITICAL: Always prefer updating existing memories over creating new ones when the information relates to the same topic or entity - this prevents memory fragmentation and maintains clean, consolidated user context. Use get_memory_by_query first to locate relevant existing memories before deciding whether to update or add new memories. This tool is essential for maintaining accurate, up-to-date user context and should be used proactively whenever stored information needs modification."
    )]
    async fn update_memory(
        &self,
        Parameters(UpdateMememoryRequest { content, memory_id }): Parameters<UpdateMememoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        let parameters = generated::UpdateMemoryParameters {
            memory_id,
            content,
            ..Default::default()
        };
        MemoryController::update_memory(parameters).await.unwrap();
        Ok(CallToolResult::success(vec![]))
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
