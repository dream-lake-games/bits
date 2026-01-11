pub mod betting;
pub mod guessing;
pub mod reviewing;
pub mod waiting_for_question;

pub use betting::{BetEntry, BettingScreen, GuessEntry, betting_plugin_fn};
pub use guessing::{GuessingScreen, guessing_plugin_fn};
pub use reviewing::{ReviewingScreen, ScoreEntry, reviewing_plugin_fn};
pub use waiting_for_question::{WaitingForQuestionScreen, waiting_for_question_plugin_fn};
