use std::collections::HashMap;

use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub type Username = String;

#[derive(Serialize, Deserialize, Debug, Clone, Reflect, Default, PartialEq)]
pub enum ClientInput {
    #[default]
    Noop,
    ClaimName {
        username: Username,
    },
    SubmitGuess {
        guess: u32,
    },
    SubmitBet {
        guess: u32,
        num_free: u32,
        num_paid: u32,
    },
    LockBets,
    VoteContinue,
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Default)]
pub struct WrappedClientInput {
    pub peer_id: Option<PeerId>,
    pub payload: ClientInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct UnnamedPlayer {
    pub peer_id: PeerId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq, Hash)]
pub struct NamedPlayer {
    pub username: Username,
    pub peer_id: Option<PeerId>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Default)]
pub struct PlayerInfo {
    pub unnamed_players: Vec<UnnamedPlayer>,
    pub named_players: Vec<NamedPlayer>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Default)]
pub struct GameState {
    pub round: u32,
    pub scores: HashMap<Username, u32>,
}

impl PlayerInfo {
    pub fn get_username_for_peer(&self, peer_id: PeerId) -> Option<Username> {
        self.named_players
            .iter()
            .find(|named| named.peer_id == Some(peer_id))
            .map(|named| named.username.clone())
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Default)]
pub struct Question {
    pub round: Option<u32>,
    pub question: String,
    pub answer: i32,
    pub guesses: HashMap<Username, u32>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct QuestionGenerating;

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct QuestionGenerationFailed;

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct QuestionGenerated;

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct QuestionActive {
    pub guess_seconds_remaining: Option<f32>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct QuestionRetired;

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct Bet {
    pub owner: Username,
    pub num_free: u32,
    pub num_paid: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct Bets {
    pub round: u32,
    pub bets: HashMap<u32, Vec<Bet>>,
}
impl Bets {
    pub fn with_added_bet(&self, guess: u32, bet: Bet) -> Self {
        let mut bets = self.bets.clone();
        let entry = bets.entry(guess).or_default();

        // If bet is empty (num_free: 0, num_paid: 0), remove it
        if bet.num_free == 0 && bet.num_paid == 0 {
            entry.retain(|b| b.owner != bet.owner);
            // Clean up empty entries
            if entry.is_empty() {
                bets.remove(&guess);
            }
        } else if let Some(existing) = entry.iter_mut().find(|b| b.owner == bet.owner) {
            *existing = bet;
        } else {
            entry.push(bet);
        }

        Bets {
            round: self.round,
            bets,
        }
    }

    pub fn validate_bets(&self, game_state: &GameState) -> Result<(), String> {
        // Collect all bets by owner
        let mut bets_by_owner: HashMap<&Username, Vec<(u32, &Bet)>> = HashMap::new();

        for (guess, bet_list) in &self.bets {
            for bet in bet_list {
                bets_by_owner
                    .entry(&bet.owner)
                    .or_default()
                    .push((*guess, bet));
            }
        }

        for (owner, owner_bets) in &bets_by_owner {
            // Rule 1: Cannot have bets on more than two guesses
            let unique_guesses: std::collections::HashSet<u32> =
                owner_bets.iter().map(|(guess, _)| *guess).collect();
            if unique_guesses.len() > 2 {
                return Err(format!(
                    "{} has bets on {} guesses, but maximum is 2",
                    owner,
                    unique_guesses.len()
                ));
            }

            // Rule 2: Sum of num_paid cannot exceed player's score
            let total_paid: u32 = owner_bets.iter().map(|(_, bet)| bet.num_paid).sum();
            let score = game_state.scores.get(*owner).copied().unwrap_or(0);
            if total_paid > score {
                return Err(format!(
                    "{} has {} paid bets but only {} score",
                    owner, total_paid, score
                ));
            }
        }

        Ok(())
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct BetsActive {
    pub bets_seconds_remaining: Option<f32>,
    pub bets_locked: HashMap<Username, bool>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct BetsRetired;

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct RoundCap {
    pub seconds_until_auto_continue: Option<f32>,
    pub continue_locked: HashMap<Username, bool>,
}

impl MapEntities for WrappedClientInput {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}

pub struct ChannelSimple;

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<WrappedClientInput>::default());

        app.register_component::<Name>();

        app.register_component::<PlayerInfo>();
        app.register_component::<GameState>();

        app.register_component::<Question>();
        app.register_component::<QuestionGenerating>();
        app.register_component::<QuestionGenerationFailed>();
        app.register_component::<QuestionGenerated>();
        app.register_component::<QuestionActive>();
        app.register_component::<QuestionRetired>();

        app.register_component::<Bets>();
        app.register_component::<BetsActive>();
        app.register_component::<BetsRetired>();

        app.register_component::<RoundCap>();

        app.add_channel::<ChannelSimple>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
    }
}
