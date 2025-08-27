# Umem - External Memory Persistence for LLMs

> **Semantic memory layer that works with every LLM via MCP (Model Context Protocol)**

Umem is a high-performance external memory system built in Rust that provides persistent, semantic memory capabilities for LLMs and AI agents. It offers memory storage, retrieval, and search through both MCP protocol and gRPC API.

## Key Features

- **Multi-tenant Memory**: Isolated memory spaces per user with OAuth authentication
- **Semantic Search**: BGE-M3 embeddings via Cloudflare Workers AI with Qdrant vector storage  
- **MCP Integration**: Native Model Context Protocol support with three memory tools
- **Document Processing**: Extract and store content from PDFs, websites, and text files
- **Real-time Performance**: Async Rust architecture with concurrent gRPC and MCP servers

## Quick Start

### Prerequisites
- Rust 1.70+
- Qdrant vector database
- Cloudflare Workers AI account  
- WorkOS account for authentication

### Environment Setup

Create `.env` file:
```env
# Vector Database
QDRANT_URL=http://localhost:6333
QDRANT_KEY=your_qdrant_key
QDRANT_COLLECTION_NAME=umem_memories

# Cloudflare Workers AI for Embeddings
CLOUDFLARE_ACCOUNT_ID=your_account_id
CLOUDFLARE_API_TOKEN=your_api_token

# OAuth Authentication
JWKS_URL=https://api.workos.com/sso/jwks/your_connection_id
WORKOS_AUTHKIT_URL=https://your-domain.workos.com
```

### Installation & Running

```bash
# Start Qdrant
docker run -d -p 6333:6333 qdrant/qdrant

# Build and run Umem
cargo build --release
cargo run --release
```

**Servers start on:**
- MCP Server: `http://127.0.0.1:3000` (OAuth protected)
- gRPC Server: `[::1]:50051`

## Usage

### MCP Tools (Primary Interface)

Umem provides three MCP tools for LLM integration:

#### 1. `add_memory`
Store new memory content:
```json
{
  "text": "Rust is a systems programming language focused on safety and performance"
}
```

#### 2. `get_memory` 
Retrieve all memories for authenticated user (no parameters required).

#### 3. `get_memory_by_query`
Semantic search across memories:
```json
{
  "query": "What programming language focuses on safety?"
}
```

### gRPC API

For programmatic access:

```rust
use umem_proto_generated::generated::*;

// Add memory
let memory = Memory {
    user_id: "user123".to_string(),
    content: "Your memory content".to_string(),
    priority: 5,
    tags: vec!["tag1".to_string()],
    ..Default::default()
};

// Search memories
let query = GetMemoriesByQueryParameters {
    user_id: "user123".to_string(),
    query: "search query".to_string(),
};
```

## Architecture

```
umem/
├── src/main.rs                    # Entry point - runs MCP + gRPC servers
├── crates/
│   ├── umem_controller/           # Business logic orchestration
│   ├── umem_mcp/                 # MCP server with OAuth authentication  
│   ├── umem_grpc_server/         # gRPC API implementation
│   ├── umem_proto_generated/     # Protocol buffer definitions
│   ├── umem_embeddings/          # Cloudflare BGE-M3 embeddings
│   ├── umem_vector/              # Qdrant vector database operations
│   ├── umem_doc_parser/          # PDF/document text extraction
│   ├── umem_web_scrapper/        # Web content scraping
│   ├── umem_search/              # Search indexing utilities
│   ├── umem_summarizer/          # Content summarization (planned)
│   └── umem_utils/               # Shared utilities
```

### Memory Data Structure

```protobuf
message Memory {
  string user_id = 1;      // Multi-tenant isolation
  string memory_id = 2;    // Unique identifier
  string content = 3;      // Text content
  int32 priority = 4;      // Priority level (1-10)  
  repeated string tags = 5; // Categorization tags
  int64 created_at = 6;    // Creation timestamp
  int64 updated_at = 7;    // Update timestamp
}
```

## Docker Deployment

```bash
docker build -t umem .
docker run -d --name umem -p 3000:3000 -p 50051:50051 --env-file .env umem
```

## Development

```bash
# Run tests
cargo test

# Run with auto-reload
cargo install cargo-watch
cargo watch -x run
```

## API Reference

### gRPC Service Methods
- `AddMemory(Memory)` - Store new memory
- `AddMemoryBulk(MemoryBulk)` - Bulk memory storage
- `UpdateMemory(UpdateMemoryParameters)` - Update existing memory
- `DeleteMemory(DeleteMemoryParameters)` - Delete memory
- `GetMemoriesByQuery(GetMemoriesByQueryParameters)` - Semantic search
- `GetMemoriesByUserID(GetMemoriesByUserIDParameters)` - Get all user memories

### MCP Tools
- **add_memory**: Store memory content  
- **get_memory**: Retrieve all user memories
- **get_memory_by_query**: Semantic memory search

## Performance Features

- **Concurrent Architecture**: Async Rust with Tokio runtime
- **Vector Optimized**: Qdrant HNSW indexing for fast similarity search
- **Efficient Embeddings**: Cloudflare Workers AI for BGE-M3 generation
- **Multi-tenant Isolation**: User-scoped memory access with OAuth

## Security

- **OAuth 2.0**: WorkOS-based authentication for MCP endpoints
- **Multi-tenant**: Strict user isolation at database level  
- **No Secrets Logging**: Secure credential handling throughout

## License

MIT License - see LICENSE file for details.

---

**Umem** - Persistent external memory for the AI era, built with Rust.