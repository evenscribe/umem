use anyhow::{bail, Result};
use lazy_static::lazy_static;
use reqwest::cookie::Jar;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

pub struct Scrapper;

lazy_static! {
    static ref jar: Arc<Jar> = Arc::new(Jar::default());
    static ref client: Client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_provider(Arc::clone(&jar))
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");
}

impl Scrapper {
    pub async fn scrape(url: &str) -> Result<String> {
        let html = Self::scrape_callback(url).await?;
        if html.contains("checking your browser")
            || html.contains("cloudflare")
            || html.contains("Please wait")
            || html.contains("ray id")
        {
            tokio::time::sleep(Duration::from_secs(5)).await;
            return Ok(Self::scrape_callback(url).await?);
        }

        Ok(html)
    }

    async fn scrape_callback(url: &str) -> Result<String> {
        let response = client
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("DNT", "1")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Cache-Control", "max-age=0")
            .send()
            .await?;
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            bail!("Couldn't be parsed. {url} returned with status-code {status}");
        }
        let html = response.text().await?;
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    async fn assert_contains(url: &str, expected: &[&str]) -> Result<String> {
        let html = Scrapper::scrape_callback(url).await?;
        for pat in expected {
            assert!(html.contains(pat), "Expected `{}` in {}", pat, url);
        }
        Ok(html)
    }

    #[tokio::test]
    async fn test_books_to_scrape_homepage() -> Result<()> {
        let url = "https://books.toscrape.com/";
        assert_contains(url, &["Books to Scrape", "All products"]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_specific_book_page() -> Result<()> {
        let url = "https://books.toscrape.com/catalogue/a-light-in-the-attic_1000/index.html";
        assert_contains(url, &["A Light in the Attic", "In stock"]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_quotes_to_scrape_homepage() -> Result<()> {
        let url = "https://quotes.toscrape.com/";
        assert_contains(url, &["Quotes to Scrape", "Albert Einstein"]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_httpbin_headers_and_user_agent() -> Result<()> {
        let url = "https://httpbin.org/headers";
        let html = Scrapper::scrape_callback(url).await?;
        assert!(html.contains("User-Agent"));
        assert!(html.contains("Mozilla/5.0")); // from our client UA
        Ok(())
    }

    #[tokio::test]
    async fn test_httpbin_cookie_roundtrip() -> Result<()> {
        // Set a cookie
        let _ =
            Scrapper::scrape_callback("https://httpbin.org/cookies/set/testkey/testvalue").await?;
        // Retrieve cookies
        let html = Scrapper::scrape_callback("https://httpbin.org/cookies").await?;
        assert!(html.contains("testkey"));
        assert!(html.contains("testvalue"));
        Ok(())
    }

    #[tokio::test]
    async fn test_openlibrary_homepage() -> Result<()> {
        let url = "https://openlibrary.org/";
        assert_contains(url, &["Open Library", "Borrow"]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_wikipedia_main_page() -> Result<()> {
        let url = "https://en.wikipedia.org/wiki/Main_Page";
        let html = Scrapper::scrape_callback(url).await?;
        assert!(html.contains("Wikipedia"));
        assert!(html.contains("From today"));
        Ok(())
    }

    #[tokio::test]
    async fn test_404_results_in_error() {
        let url = "https://httpbin.org/status/404";
        let res = Scrapper::scrape_callback(url).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_open_ai() -> Result<()> {
        let url = "https://openai.com";
        let html = Scrapper::scrape_callback(url).await?;
        assert!(html.contains("What can I help with?"));
        Ok(())
    }
}
