use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::Serialize;
mod cf_baai_bge_m3;
pub use cf_baai_bge_m3::CfBaaiBgeM3Embeder;

lazy_static! {
    static ref client: Client = Client::new();
}

#[async_trait]
pub trait Embedder {
    async fn generate_embedding(&self, text: String) -> Result<Vec<f32>>;
    async fn generate_embeddings_bulk(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

#[derive(Serialize)]
pub struct EmbeddingRequest {
    text: Vec<String>,
}
