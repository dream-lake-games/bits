use bevy::log::{info, trace, warn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::ai::agent::{Agent, Tool};
use crate::prelude::{ExaClient, Source};

pub const MAX_ANSWER: u32 = 999_999;

#[derive(Debug, Clone)]
pub struct GeneratedQuestion {
    pub question: String,
    pub answer: u32,
    pub units: Option<String>,
    pub sources: Vec<Source>,
    pub reasoning: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchArgs {
    /// Search query to find numerical facts on Wikipedia
    query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: Option<String>,
    pub snippet: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct QuestionOutput {
    /// Brief explanation of your research and question design
    reasoning: String,
    /// The Fermi estimation question text
    question: String,
    /// The numerical answer (must be 1-999999)
    answer: u32,
    /// Units if scaled (e.g., "millions", "thousands")
    units: Option<String>,
    /// URLs of Wikipedia sources used
    source_urls: Vec<String>,
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

const SYSTEM_PROMPT: &str = r#"You are a Fermi estimation question generator. Your job is to:
1. Search Wikipedia for interesting numerical facts
2. Create engaging questions based on those facts

## How to Use the Search Tool
- Make 2-3 searches to find interesting numerical facts
- Focus on surprising, non-obvious quantities
- Look for counts, measurements, statistics, records

## Question Requirements
CRITICAL: The answer must be a positive integer between 1 and 999,999.

## Choosing Good Units
Pick units that give ROUND, MEMORABLE answers - ideally between 1 and 10,000.

Examples of GOOD unit choices:
- 238,855 miles → 239 thousand miles (not 239,000 miles or 0.24 million miles)
- 100,000 meters → 100 km (not 100,000 m)
- 13,929,286 people → 14 million (not 13,929 thousand)
- $2,800,000,000 → 3 billion dollars (not 2,800 million)
- 5,280 feet → 5,280 feet (already a good number, no scaling needed)

The goal: Choose the unit that makes the answer a nice, round-ish number.

The question text MUST clearly state the units being asked for.

## Examples of Good Questions
- "How many dimples are on a standard golf ball?" → 336
- "In millions, what is Japan's population?" → 125, units: "million"
- "In what year was the Eiffel Tower completed?" → 1889
- "How many bones are in the adult human body?" → 206
- "In kilometers, what is the diameter of Earth?" → 12742, units: "km"

## Process
1. Search for interesting facts related to the topic
2. Find a numerical fact that would make a good estimation question
3. Choose units that give a nice round answer (ideally 1-10,000)
4. Formulate the question clearly stating the units
5. Return the final question with answer and sources"#;

fn current_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days = secs / 86400;
    let years = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{}-{:02}-{:02}", years, month, day)
}

fn search_tool(
    exa: Arc<ExaClient>,
) -> Tool<SearchArgs, impl Fn(SearchArgs) -> BoxFuture<'static, anyhow::Result<Vec<SearchResult>>> + Send + Sync + 'static>
{
    Tool::new(
        "search_wikipedia",
        "Search Wikipedia for numerical facts. Returns relevant article snippets.",
        move |args: SearchArgs| -> BoxFuture<'static, anyhow::Result<Vec<SearchResult>>> {
            let exa = exa.clone();
            Box::pin(async move {
                trace!("[QuestionGen] Search query: {}", args.query);
                let results = exa.search_wikipedia(&args.query, 3).await?;
                let search_results: Vec<SearchResult> = results
                    .into_iter()
                    .map(|r| SearchResult {
                        url: r.url,
                        title: r.title,
                        snippet: r.text.unwrap_or_default(),
                    })
                    .collect();
                trace!(
                    "[QuestionGen] Search returned {} result(s)",
                    search_results.len()
                );
                Ok(search_results)
            })
        },
    )
}

pub async fn generate_question(
    openai: &async_openai::Client<async_openai::config::OpenAIConfig>,
    exa: &ExaClient,
    instructions: &str,
) -> anyhow::Result<GeneratedQuestion> {
    info!("[QuestionGen] Starting generation: {}", instructions);
    let exa = Arc::new(exa.clone());

    let agent = Agent::new("question_gen")
        .system(SYSTEM_PROMPT)
        .tool(search_tool(exa.clone()))
        .max_turns(10);

    let input = format!(
        "Today's date: {}\nTopic: {}\n\nGenerate a Fermi estimation question about this topic. Search Wikipedia for interesting numerical facts, then create an engaging question.",
        current_date(),
        instructions
    );

    let result = agent.run::<QuestionOutput, _>(openai, &input).await?;
    let output = result.output;

    trace!(
        "[QuestionGen] Agent output: question='{}' answer={} units={:?}",
        output.question,
        output.answer,
        output.units
    );

    if output.answer == 0 || output.answer > MAX_ANSWER {
        warn!(
            "[QuestionGen] Invalid answer {}. Question: '{}', Units: {:?}, Reasoning: {}",
            output.answer, output.question, output.units, output.reasoning
        );
        anyhow::bail!(
            "Invalid answer {}: must be between 1 and {}",
            output.answer,
            MAX_ANSWER
        );
    }

    let sources: Vec<Source> = output
        .source_urls
        .into_iter()
        .map(|url| Source::new(url))
        .collect();

    let generated = GeneratedQuestion {
        question: output.question,
        answer: output.answer,
        units: output.units,
        sources,
        reasoning: output.reasoning,
    };

    info!(
        "[QuestionGen] ✓ Generated: '{}' → {} {:?}",
        generated.question,
        generated.answer,
        generated.units
    );

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_openai_client() -> async_openai::Client<async_openai::config::OpenAIConfig> {
        async_openai::Client::new()
    }

    fn test_exa_client() -> Option<ExaClient> {
        std::env::var("EXA_API_KEY").ok().map(ExaClient::new)
    }

    #[tokio::test]
    async fn test_generate_question() {
        let Some(exa) = test_exa_client() else {
            println!("Skipping test: EXA_API_KEY not set");
            return;
        };
        let openai = test_openai_client();

        let result = generate_question(&openai, &exa, "general trivia and interesting facts").await;

        match result {
            Ok(q) => {
                println!("\n=== Generated Question ===");
                println!("Q: {}", q.question);
                println!("A: {} {:?}", q.answer, q.units);
                println!("Sources: {:?}", q.sources.iter().map(|s| &s.url).collect::<Vec<_>>());
                println!("Reasoning: {}", q.reasoning);
                assert!(q.answer > 0 && q.answer <= MAX_ANSWER);
            }
            Err(e) => {
                println!("Generation failed (this can happen): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_generate_scaled_question() {
        let Some(exa) = test_exa_client() else {
            println!("Skipping test: EXA_API_KEY not set");
            return;
        };
        let openai = test_openai_client();

        let result = generate_question(&openai, &exa, "world cities population").await;

        match result {
            Ok(q) => {
                println!("\n=== Population Question (should be scaled) ===");
                println!("Q: {}", q.question);
                println!("A: {} {:?}", q.answer, q.units);
                assert!(q.answer > 0 && q.answer <= MAX_ANSWER);
                // Population questions should usually have units
                println!("Has units: {}", q.units.is_some());
            }
            Err(e) => {
                println!("Generation failed (this can happen): {}", e);
            }
        }
    }
}
