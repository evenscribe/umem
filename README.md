# 🧠 umem - Universal Memory Engine

> **The next-generation intelligent memory system that never forgets**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

**umem** is a blazingly fast, AI-powered memory engine built in Rust that transforms how applications store, search, and retrieve contextual information. Think of it as your application's external brain - capable of understanding, organizing, and instantly recalling any piece of information with semantic precision.

## 🚀 Why umem?

In a world drowning in data, **umem** cuts through the noise:

- **🔍 Semantic Search**: Find information by meaning, not just keywords
- **⚡ Lightning Fast**: Built in Rust for maximum performance and safety
- **🧠 AI-Powered**: Advanced embeddings and vector search capabilities
- **🔧 Developer-First**: Simple gRPC API that integrates seamlessly
- **📈 Scalable**: From prototype to production, umem grows with you
- **🎯 Multi-Modal**: Handle text, documents, PDFs, and web content

## ✨ Features

### 🎯 Core Memory Operations
- **Add Memories**: Store any text content with metadata, tags, and priorities
- **Semantic Search**: Query using natural language and get contextually relevant results
- **Bulk Operations**: Efficiently handle large datasets
- **Memory Management**: Update, delete, and organize memories with ease

### 🔧 Advanced Capabilities
- **Document Processing**: Extract and index content from PDFs and various document formats
- **Web Scraping**: Automatically extract and store content from websites
- **Vector Embeddings**: Powered by state-of-the-art BGE-M3 embeddings
- **Multi-Tenant**: Isolated memory spaces per user/application
- **Real-time Indexing**: Instant search capabilities with Tantivy integration

### 🏗️ Architecture Highlights
- **Modular Design**: Clean separation of concerns across specialized crates
- **gRPC API**: High-performance, language-agnostic communication
- **Vector Database**: Qdrant integration for similarity search
- **Async-First**: Built on Tokio for maximum concurrency

## 🏃‍♂️ Quick Start

### Prerequisites
- Rust 1.70+ 
- Docker (for Qdrant vector database)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/evenscribe/umem.git
   cd umem
   ```

2. **Start Qdrant (Vector Database)**
   ```bash
   docker run -p 6333:6333 qdrant/qdrant
   ```

3. **Set up environment**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Build and run**
   ```bash
   cargo build --release
   cargo run
   ```

The gRPC server will start on `[::1]:50051` by default.

## 🎮 Usage Examples

### Adding a Memory
```rust
use umem_proto_generated::generated::*;

let memory = Memory {
    user_id: "user123".to_string(),
    memory_id: "mem_001".to_string(),
    content: "Rust is a systems programming language focused on safety and performance".to_string(),
    priority: 5,
    tags: vec!["programming".to_string(), "rust".to_string()],
    created_at: chrono::Utc::now().timestamp(),
    updated_at: chrono::Utc::now().timestamp(),
};

// Add via gRPC client
client.add_memory(memory).await?;
```

### Semantic Search
```rust
let query = GetMemoriesByQueryParameters {
    user_id: "user123".to_string(),
    query: "What programming language focuses on safety?".to_string(),
};

let results = client.get_memories_by_query(query).await?;
// Returns semantically similar memories about Rust!
```

### Document Processing
```rust
use umem_doc_parser::{Extractor, FileExtractionSource};

// Extract text from PDF
let content = Extractor::extract_from_file("document.pdf", FileExtractionSource::PDF)?;

// Extract from website
let web_content = Extractor::extract_from_website("https://example.com").await?;
```

## 🏗️ Architecture

umem is built as a modular workspace with specialized crates:

```
umem/
├── src/main.rs                 # Main application entry point
├── crates/
│   ├── umem_controller/        # High-level orchestration logic
│   ├── umem_grpc_server/       # gRPC service implementation
│   ├── umem_proto_generated/   # Protocol buffer definitions
│   ├── umem_search/            # Search indexing and retrieval
│   ├── umem_embeddings/        # AI embedding generation
│   ├── umem_vector/            # Vector database operations
│   ├── umem_doc_parser/        # Document content extraction
│   ├── umem_web_scrapper/      # Web content extraction
│   └── umem_summarizer/        # Content summarization
```

### Data Flow
1. **Input**: Content arrives via gRPC API
2. **Processing**: Documents are parsed, text is extracted
3. **Embedding**: Content is converted to vector embeddings
4. **Storage**: Vectors stored in Qdrant, metadata in search index
5. **Retrieval**: Semantic queries find similar vectors
6. **Response**: Relevant memories returned with context

## 🔧 Configuration

Create a `.env` file in the project root:

```env
# Qdrant Configuration
QDRANT_URL=http://localhost:6333
QDRANT_COLLECTION_NAME=umem_memories

# Embedding Service
EMBEDDING_MODEL_URL=your_embedding_service_url
EMBEDDING_API_KEY=your_api_key

# Server Configuration
GRPC_SERVER_ADDRESS=[::1]:50051
LOG_LEVEL=info
```

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p umem_doc_parser

# Run with output
cargo test -- --nocapture
```

## 📊 Performance

umem is designed for speed and efficiency:

- **Sub-millisecond** memory retrieval for cached queries
- **Concurrent processing** of multiple requests
- **Efficient vector operations** with optimized embeddings
- **Memory-safe** operations with zero-cost abstractions

## 🛣️ Roadmap

### 🎯 Current Focus (v0.1.x)
- [x] Core memory operations (CRUD)
- [x] Semantic search with embeddings
- [x] Document parsing (PDF, text, markdown)
- [x] Web content extraction
- [x] gRPC API foundation

### 🚀 Coming Soon (v0.2.x)
- [ ] Advanced summarization capabilities
- [ ] Multi-modal embeddings (text + images)
- [ ] Real-time memory updates
- [ ] Advanced filtering and faceted search
- [ ] Memory clustering and organization

### 🌟 Future Vision (v1.0+)
- [ ] Distributed memory clusters
- [ ] GraphQL API alongside gRPC
- [ ] Advanced AI reasoning over memories
- [ ] Plugin system for custom processors
- [ ] Web dashboard for memory management

## 🤝 Contributing

We welcome contributions! umem is built by developers, for developers.

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes** and add tests
4. **Run the test suite**: `cargo test`
5. **Submit a pull request**

### Development Setup
```bash
# Install development dependencies
cargo install cargo-watch
cargo install cargo-tarpaulin  # For coverage

# Run with auto-reload
cargo watch -x run

# Generate coverage report
cargo tarpaulin --out html
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Qdrant** for the excellent vector database
- **Tantivy** for blazing-fast full-text search
- **BGE-M3** for state-of-the-art embeddings
- **Rust Community** for the amazing ecosystem

## 📞 Support & Community

- 🐛 **Issues**: [GitHub Issues](https://github.com/evenscribe/umem/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/evenscribe/umem/discussions)

---

<div align="center">

**Built with ❤️ in Rust**

*umem - Because every application deserves a perfect memory*

[⭐ Star us on GitHub](https://github.com/evenscribe/umem)
</div>
