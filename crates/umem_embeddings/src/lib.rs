use reqwest::Client;
use serde::{Deserialize, Serialize};

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

    /// Generates text embeddings for the provided input texts using a remote API.
    ///
    /// Sends the input texts to the configured embedding model and returns the resulting embedding vectors.
    ///
    /// # Arguments
    ///
    /// * `texts` - A vector of input strings to generate embeddings for.
    ///
    /// # Returns
    ///
    /// A vector of embedding vectors, where each inner vector corresponds to the embedding of an input text.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or if the response cannot be deserialized.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() -> anyhow::Result<()> {
    /// let generator = EmbeddingsGenerator::new(
    ///     "model_name".to_string(),
    ///     "account_id".to_string(),
    ///     "api_token".to_string(),
    /// );
    /// let texts = vec![
    ///     "Hello, world!".to_string(),
    ///     "How are you?".to_string(),
    /// ];
    /// let embeddings = generator.generate_embeddings(texts).await?;
    /// assert_eq!(embeddings.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
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

        Ok(embedding_response.result.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn summarize_text() {
        let account_id = match std::env::var("ACCOUNT_ID") {
            Ok(id) => id,
            Err(_) => panic!("ACCOUNT_ID environment variable not set"),
        };

        let api_key = match std::env::var("API_KEY") {
            Ok(key) => key,
            Err(_) => panic!("API_KEY environment variable not set"),
        };

        let embedder = EmbeddingsGenerator::new("@cf/baai/bge-m3".to_string(), account_id, api_key);

        let text = vec![
            "This is a story about an orange cloud".to_string(),
            "This is a story about a llama".to_string(),
            "This is a story about a hugging emoji".to_string(),
        ];

        let result = embedder.generate_embeddings(text).await;

        assert!(result.is_ok(), "Embeddings failed: {:?}", result.err());
        println!("Summary: {:?}", result.unwrap());
    }
}
