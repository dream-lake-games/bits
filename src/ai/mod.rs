pub mod agent;
mod fermi;
pub mod question_gen;

pub use agent::{Agent, AgentOutput, Tool};
pub use fermi::*;
pub use question_gen::{generate_question, GeneratedQuestion, MAX_ANSWER};

use bevy::prelude::*;
use std::collections::HashMap;

use crate::exa::ExaClient;

#[derive(Resource, Clone)]
pub struct AIClients {
    pub openai: async_openai::Client<async_openai::config::OpenAIConfig>,
    pub exa: Option<ExaClient>,
}

#[derive(Debug, Clone)]
pub struct AIGuess {
    pub guess: u32,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub enum AIInvalidGuessReason {
    NotPositive,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct AIInvalidGuess {
    pub guess: AIGuess,
    pub reason: AIInvalidGuessReason,
}

#[derive(Debug, Clone)]
pub struct AIBets {
    pub bets: HashMap<u32, u32>,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub enum AIInvalidBetsReason {
    InvalidGuessValue(String),
    DidNotUseFreeChips,
    ValidationFailed(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct AIInvalidBets {
    pub bets: AIBets,
    pub reason: AIInvalidBetsReason,
}

#[derive(Debug, Clone)]
pub struct BettingContext {
    pub question: String,
    pub answer: u32,
    pub guesses: HashMap<String, u32>,
    pub my_score: u32,
}

#[expect(async_fn_in_trait)]
pub trait AI: Clone + Send + Sync + 'static {
    async fn make_guess(
        self,
        clients: AIClients,
        question: String,
        invalid_guesses: Vec<AIInvalidGuess>,
    ) -> anyhow::Result<AIGuess>;

    async fn make_bets(
        self,
        clients: AIClients,
        context: BettingContext,
        invalid_bets: Vec<AIInvalidBets>,
    ) -> anyhow::Result<AIBets>;
}
