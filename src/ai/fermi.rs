use std::collections::HashMap;

use bevy::log::{debug, trace, warn};
use bevy::prelude::Reflect;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub struct FermiControl {}

const FERMI_SYSTEM_PROMPT: &str = r#"You are an expert at Fermi estimation. Your goal is to make quick, reasonable estimates using order-of-magnitude reasoning.

Be concise. Break down problems into 3-5 key factors, make simple assumptions, and calculate."#;

const BETTING_SYSTEM_PROMPT: &str = r#"You are a strategic bettor in a Fermi estimation game. Analyze the guesses and make smart betting decisions."#;

#[derive(Debug, Serialize, Deserialize)]
struct FermiGuessOutput {
    reasoning: String,
    guess: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct FermiBetsOutput {
    reasoning: String,
    bets: Vec<BetEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BetEntry {
    guess_value: u32,
    chips: u32,
}

fn build_guess_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "reasoning": {
                "type": "string",
                "description": "Brief step-by-step reasoning (3-5 bullet points)"
            },
            "guess": {
                "type": "integer",
                "description": "Your final estimate (positive integer > 0)"
            }
        },
        "required": ["reasoning", "guess"],
        "additionalProperties": false
    })
}

fn build_bets_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "reasoning": {
                "type": "string",
                "description": "Brief explanation of betting strategy"
            },
            "bets": {
                "type": "array",
                "description": "Your bets (must total at least 2 chips)",
                "items": {
                    "type": "object",
                    "properties": {
                        "guess_value": {
                            "type": "integer",
                            "description": "Guess value to bet on (0 for lowball)"
                        },
                        "chips": {
                            "type": "integer",
                            "description": "Number of chips to bet"
                        }
                    },
                    "required": ["guess_value", "chips"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["reasoning", "bets"],
        "additionalProperties": false
    })
}

fn build_guess_prompt(question: &str, invalid_guesses: &[AIInvalidGuess]) -> String {
    let mut prompt = format!(
        r#"Question: {}

Estimate format:
• Factor 1: [assumption] → [number]
• Factor 2: [assumption] → [number]
• Factor 3: [assumption] → [number]
• Calculation: [show math]
• Final estimate: [number]

Example for "How many gas stations in the US?":
• US population: ~330 million
• People per car: ~2 → 165M cars
• Cars per gas station: ~1000
• Calculation: 165M / 1000 = 165,000
• Final estimate: 165000

Now estimate the answer."#,
        question
    );

    if !invalid_guesses.is_empty() {
        prompt.push_str("\n\nPrevious invalid guesses:");
        for invalid in invalid_guesses {
            prompt.push_str(&format!(
                "\n- {} was invalid: {:?}",
                invalid.guess.guess, invalid.reason
            ));
        }
    }

    prompt
}

fn build_bets_prompt(context: &BettingContext, invalid_bets: &[AIInvalidBets]) -> String {
    let mut prompt = format!(
        "Question: {}\nActual answer: {}\n\n",
        context.question, context.answer
    );

    prompt.push_str("Player guesses:\n");
    for (player, guess) in &context.guesses {
        prompt.push_str(&format!("- {}: {}\n", player, guess));
    }

    prompt.push_str(&format!("\nYour current score: {}\n", context.my_score));

    prompt.push_str("\nValid bet targets (guess values you can bet on):\n");
    let mut valid_values: Vec<u32> = context.guesses.values().copied().collect();
    valid_values.push(0);
    valid_values.sort();
    valid_values.dedup();
    for v in &valid_values {
        if *v == 0 {
            prompt.push_str("- 0 (lowball)\n");
        } else {
            prompt.push_str(&format!("- {}\n", v));
        }
    }

    let guesses_at_or_below: Vec<u32> = context
        .guesses
        .values()
        .filter(|&&v| v <= context.answer)
        .copied()
        .collect();

    prompt.push_str(
        r#"
WINNING RULES (CRITICAL):
- A guess can ONLY win if guess_value <= actual_answer
- If a guess is OVER the answer, betting on it is GUARANTEED TO LOSE
- The winning bet is the HIGHEST guess that is still <= answer
- Lowball (0) wins ONLY if ALL guesses are > answer (nobody got at or under)

BETTING RULES:
1. You have 2 FREE chips that you MUST use. Put both on one bet or split them.
2. You can bet on up to 2 different guess values.
3. Beyond free chips, you MAY add "paid" chips from your score (risky - you lose them if wrong).

STRATEGY:
- FIRST: Identify which guesses are <= answer (can win) vs > answer (cannot win)
- If ALL guesses are > answer, bet on lowball (0)
- Otherwise, bet on the highest guess that is <= answer
- Paid chips are risky. Only use when very confident or desperate.
- Never split paid chips across multiple bets.

"#,
    );

    if guesses_at_or_below.is_empty() {
        prompt.push_str("⚠️ NOTE: ALL guesses are OVER the answer! Lowball (0) will WIN.\n");
    } else {
        let max_winning = guesses_at_or_below.iter().max().unwrap();
        prompt.push_str(&format!(
            "✓ Guesses at or below answer: {:?}. Highest winning guess: {}\n",
            guesses_at_or_below, max_winning
        ));
    }

    prompt.push_str("\nReturn your bets. Total chips must be at least 2.");

    if !invalid_bets.is_empty() {
        prompt.push_str("\n\nYour previous bets were INVALID:");
        for invalid in invalid_bets {
            prompt.push_str(&format!(
                "\n- {:?} failed: {:?}",
                invalid.bets.bets, invalid.reason
            ));
        }
        prompt.push_str("\n\nFix your bets!");
    }

    prompt
}

impl AI for FermiControl {
    async fn make_guess(
        self,
        clients: AIClients,
        question: String,
        invalid_guesses: Vec<AIInvalidGuess>,
    ) -> anyhow::Result<AIGuess> {
        use async_openai::types::responses::{
            CreateResponseArgs, ResponseFormatJsonSchema, ResponseTextParam,
            TextResponseFormatConfiguration,
        };

        debug!("[AI Guess] Starting for question");
        if !invalid_guesses.is_empty() {
            debug!(
                "[AI Guess] {} prior invalid attempt(s)",
                invalid_guesses.len()
            );
        }

        let prompt = build_guess_prompt(&question, &invalid_guesses);
        trace!("[AI Guess] Full prompt:\n{}", prompt);

        let text_config = ResponseTextParam {
            format: TextResponseFormatConfiguration::JsonSchema(ResponseFormatJsonSchema {
                name: "fermi_guess".to_string(),
                description: Some("A Fermi estimation guess with reasoning".to_string()),
                schema: Some(build_guess_schema()),
                strict: Some(true),
            }),
            verbosity: None,
        };

        let request = CreateResponseArgs::default()
            .model("gpt-4.1-mini")
            .instructions(FERMI_SYSTEM_PROMPT)
            .input(prompt)
            .text(text_config)
            .build()?;

        let response = clients.openai.responses().create(request).await?;

        let output_text = response
            .output_text()
            .ok_or_else(|| anyhow::anyhow!("No text output in response"))?;

        trace!("[AI Guess] Raw response: {}", output_text);
        let parsed: FermiGuessOutput = match serde_json::from_str(&output_text) {
            Ok(p) => p,
            Err(e) => {
                warn!("[AI Guess] Failed to parse response: {}", e);
                warn!("[AI Guess] Raw output: {}", output_text);
                return Err(e.into());
            }
        };
        debug!("[AI Guess] → {}", parsed.guess);
        trace!("[AI Guess] Reasoning: {}", parsed.reasoning);

        Ok(AIGuess {
            guess: parsed.guess,
            reasoning: parsed.reasoning,
        })
    }

    async fn make_bets(
        self,
        clients: AIClients,
        context: BettingContext,
        invalid_bets: Vec<AIInvalidBets>,
    ) -> anyhow::Result<AIBets> {
        use async_openai::types::responses::{
            CreateResponseArgs, ResponseFormatJsonSchema, ResponseTextParam,
            TextResponseFormatConfiguration,
        };

        debug!(
            "[AI Bets] Starting (answer={}, score={}, {} guesses)",
            context.answer,
            context.my_score,
            context.guesses.len()
        );
        if !invalid_bets.is_empty() {
            debug!("[AI Bets] {} prior invalid attempt(s)", invalid_bets.len());
        }
        trace!("[AI Bets] Guesses: {:?}", context.guesses);

        let prompt = build_bets_prompt(&context, &invalid_bets);
        trace!("[AI Bets] Full prompt:\n{}", prompt);

        let text_config = ResponseTextParam {
            format: TextResponseFormatConfiguration::JsonSchema(ResponseFormatJsonSchema {
                name: "fermi_bets".to_string(),
                description: Some("Betting strategy with reasoning".to_string()),
                schema: Some(build_bets_schema()),
                strict: Some(true),
            }),
            verbosity: None,
        };

        let request = CreateResponseArgs::default()
            .model("gpt-4.1-mini")
            .instructions(BETTING_SYSTEM_PROMPT)
            .input(prompt)
            .text(text_config)
            .build()?;

        let response = clients.openai.responses().create(request).await?;

        let output_text = response
            .output_text()
            .ok_or_else(|| anyhow::anyhow!("No text output in response"))?;

        trace!("[AI Bets] Raw response: {}", output_text);
        let parsed: FermiBetsOutput = match serde_json::from_str(&output_text) {
            Ok(p) => p,
            Err(e) => {
                warn!("[AI Bets] Failed to parse response: {}", e);
                warn!("[AI Bets] Raw output: {}", output_text);
                return Err(e.into());
            }
        };

        let bets: HashMap<u32, u32> = parsed
            .bets
            .into_iter()
            .map(|entry| (entry.guess_value, entry.chips))
            .collect();

        debug!("[AI Bets] → {:?}", bets);
        trace!("[AI Bets] Reasoning: {}", parsed.reasoning);

        Ok(AIBets {
            bets,
            reasoning: parsed.reasoning,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> AIClients {
        AIClients {
            openai: async_openai::Client::new(),
            exa: None,
        }
    }

    #[tokio::test]
    async fn test_make_guess() {
        let fermi = FermiControl {};
        let result = fermi
            .make_guess(
                test_client(),
                "How many piano tuners are there in Chicago?".to_string(),
                vec![],
            )
            .await;

        match result {
            Ok(guess) => {
                println!("Guess: {}", guess.guess);
                println!("Reasoning: {}", guess.reasoning);
                assert!(guess.guess > 0);
            }
            Err(e) => println!("API call failed (expected if no key): {}", e),
        }
    }

    #[tokio::test]
    async fn test_make_bets() {
        let guesses: HashMap<String, u32> = [("Player1", 100u32), ("Player2", 500), ("AI", 250)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        let context = BettingContext {
            question: "How many piano tuners in Chicago?".to_string(),
            answer: 290,
            guesses,
            my_score: 5,
        };

        let fermi = FermiControl {};
        let result = fermi.make_bets(test_client(), context, vec![]).await;

        match result {
            Ok(bets) => {
                let filtered: HashMap<u32, u32> =
                    bets.bets.into_iter().filter(|(_, v)| *v > 0).collect();
                println!("Bets: {:?}", filtered);
                println!("Reasoning: {}", bets.reasoning);
                let total: u32 = filtered.values().sum();
                assert!(total >= 2);
            }
            Err(e) => println!("API call failed (expected if no key): {}", e),
        }
    }

    #[tokio::test]
    async fn test_lowball_scenario() {
        // All guesses are over the answer - should bet on lowball (0)
        let guesses: HashMap<String, u32> =
            [("Player1", 10_000u32), ("Player2", 8_000), ("AI", 6_000)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();

        let context = BettingContext {
            question: "How many dentists in NYC?".to_string(),
            answer: 5_000, // All guesses are over
            guesses,
            my_score: 2,
        };

        let fermi = FermiControl {};
        let result = fermi.make_bets(test_client(), context, vec![]).await;

        match result {
            Ok(bets) => {
                let filtered: HashMap<u32, u32> =
                    bets.bets.into_iter().filter(|(_, v)| *v > 0).collect();
                println!("Bets (should include lowball=0): {:?}", filtered);
                println!("Reasoning: {}", bets.reasoning);
            }
            Err(e) => println!("API call failed (expected if no key): {}", e),
        }
    }
}
