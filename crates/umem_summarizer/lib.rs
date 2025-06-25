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
    /// Initializes an HTTP client and stores the provided credentials for use in API requests.
    ///
    /// # Parameters
    /// - `model_name`: The identifier of the AI summarization model to use.
    /// - `account_id`: The Cloudflare account ID associated with the API.
    /// - `api_token`: The API token for authenticating requests.
    ///
    /// # Returns
    /// A `Summarizer` configured with the given model and credentials.
    pub fn new(model_name: String, account_id: String, api_token: String) -> Self {
        let client = Client::new();

        Self {
            client,
            model_name,
            account_id,
            api_token,
        }
    }

    /// Sends text to the Cloudflare AI summarization API and returns the summarized result.
    ///
    /// Returns an error if the API indicates failure or if the request cannot be completed.
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
    /// let summary = summarizer.summarize(
    ///     "Unicorns are mythical creatures that have been described since antiquity.",
    ///     10,
    /// ).await?;
    /// println!("{}", summary.summary);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn summarize(
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
        let account_id = match std::env::var("ACCOUNT_ID") {
            Ok(id) => id,
            Err(_) => panic!("ACCOUNT_ID environment variable not set"),
        };

        let api_key = match std::env::var("API_KEY") {
            Ok(key) => key,
            Err(_) => panic!("API_KEY environment variable not set"),
        };

        let summarizer = Summarizer::new(
            "@cf/facebook/bart-large-cnn".to_string(),
            account_id,
            api_key,
        );

        let text = "In a shocking finding, scientist discovered a herd of unicorns living in a remote, previously unexplored valley, in the Andes Mountains. Even more surprising to the researchers was the fact that the unicorns spoke perfect English.".to_string();

        let max_length = 10;

        let result = summarizer.summarize(text, max_length).await;
        assert!(result.is_ok(), "Summarization failed: {:?}", result.err());
        println!("Summary: {}", result.unwrap().summary);
    }
}
