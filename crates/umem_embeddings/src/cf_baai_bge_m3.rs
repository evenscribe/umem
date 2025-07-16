use crate::{client, Embedder};
use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest<'em> {
    text: Vec<&'em str>,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    result: EmbeddingResult,
    errors: Vec<String>,
    success: bool,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResult {
    data: Vec<Vec<f32>>,
}

const CF_BAAI_BGE_M3_EMBEDER_NAME: &str = "@cf/baai/bge-m3";

pub struct CfBaaiBgeM3Embeder {
    model_name: &'static str,
    account_id: String,
    api_token: String,
}

impl CfBaaiBgeM3Embeder {
    pub fn new(account_id: String, api_token: String) -> Self {
        Self {
            model_name: CF_BAAI_BGE_M3_EMBEDER_NAME,
            account_id,
            api_token,
        }
    }
}

#[async_trait]
impl Embedder for CfBaaiBgeM3Embeder {
    async fn generate_embedding<'em>(&self, text: &'em str) -> Result<Vec<f32>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
            self.account_id, self.model_name
        );
        let request_body = EmbeddingRequest { text: vec![text] };
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request_body)
            .send()
            .await?;
        let mut embedding_response: EmbeddingResponse = response.json().await?;
        if !embedding_response.success {
            bail!("{:?}", embedding_response.errors);
        }
        Ok(std::mem::take(&mut embedding_response.result.data[0]))
    }

    async fn generate_embeddings_bulk<'em>(&self, texts: Vec<&'em str>) -> Result<Vec<Vec<f32>>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
            self.account_id, self.model_name
        );
        let request_body = EmbeddingRequest { text: texts };
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request_body)
            .send()
            .await?;
        let embedding_response: EmbeddingResponse = response.json().await?;
        if !embedding_response.success {
            bail!("{:?}", embedding_response.errors);
        }
        Ok(embedding_response.result.data)
    }
}
