use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest {
    text: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    result: EmbeddingResult,
    errors: Vec<String>,
    success: bool,
}

#[derive(Deserialize, Debug)]
struct EmbeddingUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResult {
    shape: Vec<usize>,
    data: Vec<Vec<f32>>,
    usage: EmbeddingUsage,
    pooling: String,
}

pub struct EmbeddingsGenerator {
    client: Client,
    model_name: String,
    account_id: String,
    api_token: String,
}

impl EmbeddingsGenerator {
    pub fn new(model_name: String, account_id: String, api_token: String) -> Self {
        let client = Client::new();

        Self {
            client,
            model_name,
            account_id,
            api_token,
        }
    }

    pub async fn generate_embeddings(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
            self.account_id, self.model_name
        );

        let request_body = EmbeddingRequest { text: texts };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request_body)
            .send()
            .await?;

        let embedding_response: EmbeddingResponse = response.json().await?;
        dbg!("Embedding response: {:?}", &embedding_response);

        Ok(embedding_response.result.data)
    }
}
