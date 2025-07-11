use crate::{client, Embedder};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

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

        let request_body = [("text", vec![text])];

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request_body)
            .send()
            .await?;

        let embedding_response: HashMap<String, String> = response.json().await?;

        if !embedding_response
            .get("success")
            .unwrap_or(&"false".to_string())
            .parse::<bool>()?
        {
            if let Some(errors) = embedding_response.get("errors") {
                if !errors.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Embedding service returned errors: {:?}",
                        errors
                    ));
                }
            }
            return Err(anyhow::anyhow!("Embedding service returned success=false"));
        }

        let result = embedding_response.get("result").unwrap();

        let result = serde_json::from_str::<HashMap<String, String>>(result)
            .map_err(|e| anyhow::anyhow!("Failed to parse 'result': {}", e))?;

        if !result.contains_key("data") {
            return Err(anyhow::anyhow!("Missing 'data' in result"));
        }

        let mut embedding_response: Vec<Vec<f32>> =
            serde_json::from_str(result.get("data").unwrap())
                .map_err(|e| anyhow::anyhow!("Failed to parse 'data': {}", e))?;

        Ok(std::mem::take(&mut embedding_response[0]))
    }

    async fn generate_embeddings_bulk<'em>(&self, texts: Vec<&'em str>) -> Result<Vec<Vec<f32>>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
            self.account_id, self.model_name
        );

        let request_body = [("text", texts)];

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request_body)
            .send()
            .await?;

        let embedding_response: HashMap<String, String> = response.json().await?;

        if !embedding_response
            .get("success")
            .unwrap_or(&"false".to_string())
            .parse::<bool>()?
        {
            if let Some(errors) = embedding_response.get("errors") {
                if !errors.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Embedding service returned errors: {:?}",
                        errors
                    ));
                }
            }
            return Err(anyhow::anyhow!("Embedding service returned success=false"));
        }

        let result = embedding_response.get("result").unwrap();

        let result = serde_json::from_str::<HashMap<String, String>>(result)
            .map_err(|e| anyhow::anyhow!("Failed to parse 'result': {}", e))?;

        if !result.contains_key("data") {
            return Err(anyhow::anyhow!("Missing 'data' in result"));
        }

        let embedding_response: Vec<Vec<f32>> =
            serde_json::from_str(result.get("data").unwrap())
                .map_err(|e| anyhow::anyhow!("Failed to parse 'data': {}", e))?;

        Ok(embedding_response)
    }
}
