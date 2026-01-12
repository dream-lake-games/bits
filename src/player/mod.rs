//! Player screen components - interactive UI for player phones.

pub mod betting;
pub mod guessing;
pub mod reviewing;
pub mod waiting;

pub use betting::{
    BettingGuessDisplay, PendingBet, PlayerBettingScreen, player_betting_plugin_fn,
};
pub use guessing::{PlayerGuessingScreen, player_guessing_plugin_fn};
pub use reviewing::{PlayerReviewingScreen, player_reviewing_plugin_fn};
pub use waiting::{PlayerWaitingScreen, player_waiting_plugin_fn};

