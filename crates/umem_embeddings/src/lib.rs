use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref client: Client = Client::new();
}

#[async_trait]
trait Embedder {
    async fn generate_embedding(&self, text: String) -> Result<Vec<f32>>;
    async fn generate_embeddings_bulk(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

#[derive(Serialize)]
pub struct EmbeddingRequest {
    text: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct EmbeddingResponse {
    result: EmbeddingResult,
    errors: Vec<String>,
    success: bool,
}

#[derive(Deserialize, Debug)]
pub struct EmbeddingUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Deserialize, Debug)]
pub struct EmbeddingResult {
    shape: Vec<usize>,
    data: Vec<Vec<f32>>,
    usage: EmbeddingUsage,
    pooling: String,
}

mod cf_baai_bge_m3;
pub use cf_baai_bge_m3::CfBaaiBgeM3Embeder;
