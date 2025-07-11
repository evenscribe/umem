use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use reqwest::Client;
mod cf_baai_bge_m3;
pub use cf_baai_bge_m3::CfBaaiBgeM3Embeder;

lazy_static! {
    static ref client: Client = Client::new();
}

#[async_trait]
pub trait Embedder {
    async fn generate_embedding<'em>(&self, text: &'em str) -> Result<Vec<f32>>;
    async fn generate_embeddings_bulk<'em>(&self, texts: Vec<&'em str>) -> Result<Vec<Vec<f32>>>;
}
