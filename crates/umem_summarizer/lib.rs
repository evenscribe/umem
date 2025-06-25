use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SummarizationRequest {
    input_text: String,
    max_length: usize,
}

#[derive(Deserialize, Debug)]
pub struct SummarizationResponse {
    pub(crate) result: SummarizationResult,
    pub(crate) errors: Vec<String>,
    pub(crate) success: bool,
}

#[derive(Deserialize, Debug)]
pub struct SummarizationResult {
    pub summary: String,
}

pub struct Summarizer {
    pub(crate) client: Client,
    pub(crate) model_name: String,
    pub(crate) account_id: String,
    pub(crate) api_token: String,
}

impl Summarizer {
    /// Creates a new `Summarizer` instance with the specified model name, account ID, and API token.
    ///
    /// Initializes the internal HTTP client and stores the provided credentials for API interaction.
    ///
    /// # Parameters
    /// - `model_name`: The identifier of the summarization model to use.
    /// - `account_id`: The Cloudflare account ID associated with the API.
    /// - `api_token`: The API token for authentication.
    ///
    /// # Returns
    /// A `Summarizer` configured with the given model and credentials.
    ///
    /// # Examples
    ///
    /// ```
    /// let summarizer = Summarizer::new(
    ///     "@cf/facebook/bart-large-cnn".to_string(),
    ///     "your_account_id".to_string(),
    ///     "your_api_token".to_string(),
    /// );
    /// ```
    pub fn new(model_name: String, account_id: String, api_token: String) -> Self {
        let client = Client::new();

        Self {
            client,
            model_name,
            account_id,
            api_token,
        }
    }

    /// Sends a request to the remote summarization API and returns the generated summary.
    ///
    /// This asynchronous method posts the provided text and maximum summary length to the configured summarization model.
    /// If the API call is successful, it returns the summary result; otherwise, it returns an error containing the API's error messages.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to be summarized.
    /// * `max_length` - The maximum length of the generated summary.
    ///
    /// # Returns
    ///
    /// A `SummarizationResult` containing the summary text on success, or an error if the API request fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use umem_summarizer::{Summarizer, SummarizationResult};
    /// # async fn example() -> anyhow::Result<()> {
    /// let summarizer = Summarizer::new(
    ///     "@cf/facebook/bart-large-cnn".to_string(),
    ///     "your_account_id".to_string(),
    ///     "your_api_token".to_string(),
    /// );
    /// let result: SummarizationResult = summarizer
    ///     .generate_embeddings("This is a long article that needs summarization.".to_string(), 10)
    ///     .await?;
    /// println!("{}", result.summary);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_embeddings(
        &self,
        text: String,
        max_length: usize,
    ) -> anyhow::Result<SummarizationResult> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
            self.account_id, self.model_name
        );

        let request_body = SummarizationRequest {
            input_text: text,
            max_length,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request_body)
            .send()
            .await?;

        let summarization_response: SummarizationResponse = response.json().await?;

        if !summarization_response.success {
            return Err(anyhow::anyhow!(
                "Summarization failed: {}",
                summarization_response.errors.join(", ")
            ));
        }

        Ok(summarization_response.result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn summarize_text() {
        let account_id = dbg!(match std::env::var("ACCOUNT_ID") {
            Ok(id) => id,
            Err(_) => panic!("ACCOUNT_ID environment variable not set"),
        });

        let api_key = dbg!(match std::env::var("API_KEY") {
            Ok(key) => key,
            Err(_) => panic!("API_KEY environment variable not set"),
        });

        let summarizer = Summarizer::new(
            "@cf/facebook/bart-large-cnn".to_string(),
            account_id,
            api_key,
        );

        let text = "In a shocking finding, scientist discovered a herd of unicorns living in a remote, previously unexplored valley, in the Andes Mountains. Even more surprising to the researchers was the fact that the unicorns spoke perfect English.".to_string();

        let max_length = 10;

        let result = summarizer
            .generate_embeddings(text, max_length)
            .await
            .unwrap();

        dbg!(result);
    }
}
