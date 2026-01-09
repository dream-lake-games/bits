use serde::{Deserialize, Serialize};

use crate::prelude::Source;

#[derive(Clone)]
pub struct ExaClient {
    http: reqwest::Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct ExaSearchRequest {
    query: String,
    #[serde(rename = "includeDomains")]
    include_domains: Vec<String>,
    #[serde(rename = "numResults")]
    num_results: u32,
    contents: ExaContentsConfig,
}

#[derive(Debug, Serialize)]
struct ExaContentsConfig {
    text: ExaTextConfig,
}

#[derive(Debug, Serialize)]
struct ExaTextConfig {
    #[serde(rename = "maxCharacters")]
    max_characters: u32,
}

#[derive(Debug, Deserialize)]
struct ExaSearchResponse {
    results: Vec<ExaSearchResult>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExaSearchResult {
    pub url: String,
    pub title: Option<String>,
    pub text: Option<String>,
}

impl From<&ExaSearchResult> for Source {
    fn from(result: &ExaSearchResult) -> Self {
        let mut source = Source::new(&result.url);
        if let Some(title) = &result.title {
            source = source.with_title(title);
        }
        if let Some(text) = &result.text {
            let snippet: String = text.chars().take(500).collect();
            source = source.with_snippet(snippet);
        }
        source
    }
}

const DEFAULT_TEXT_MAX_CHARS: u32 = 5000;

impl ExaClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
        }
    }

    pub async fn search_wikipedia(
        &self,
        query: &str,
        num_results: u32,
    ) -> anyhow::Result<Vec<ExaSearchResult>> {
        self.search(query, num_results, vec!["wikipedia.org".to_string()])
            .await
    }

    pub async fn search(
        &self,
        query: &str,
        num_results: u32,
        include_domains: Vec<String>,
    ) -> anyhow::Result<Vec<ExaSearchResult>> {
        let request = ExaSearchRequest {
            query: query.to_string(),
            include_domains,
            num_results,
            contents: ExaContentsConfig {
                text: ExaTextConfig {
                    max_characters: DEFAULT_TEXT_MAX_CHARS,
                },
            },
        };

        let response = self
            .http
            .post("https://api.exa.ai/search")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Exa API error: {} - {}", status, body);
        }

        let result: ExaSearchResponse = response.json().await?;
        Ok(result.results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> Option<ExaClient> {
        std::env::var("EXA_API_KEY").ok().map(ExaClient::new)
    }

    #[tokio::test]
    async fn test_search_wikipedia() {
        let Some(client) = test_client() else {
            eprintln!("Skipping test: EXA_API_KEY not set");
            return;
        };

        let results = client
            .search_wikipedia("Minnesota Vikings Super Bowl appearances", 3)
            .await;

        match results {
            Ok(results) => {
                println!("Found {} results:", results.len());
                for result in &results {
                    println!(
                        "  - {} ({})",
                        result.title.as_deref().unwrap_or("?"),
                        result.url
                    );
                    if let Some(text) = &result.text {
                        println!("    Text length: {} chars", text.len());
                        let preview: String = text.chars().take(200).collect();
                        println!("    Preview: {}...", preview);
                    }
                }
                assert!(!results.is_empty());
                assert!(results.iter().all(|r| r.url.contains("wikipedia.org")));
            }
            Err(e) => {
                panic!("Search failed: {}", e);
            }
        }
    }
}
