use std::collections::HashMap;

use crate::prelude::*;
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub type Username = String;

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq, Default)]
pub struct Source {
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
}

impl Source {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: None,
            snippet: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }
}

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
    RequestHost,
    StartGame,
    AddAI,
    RemoveAI { username: Username },
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Default)]
pub struct WrappedClientInput {
    pub peer_id: Option<PeerId>,
    pub payload: ClientInput,
}

/// Input for game systems, decoupled from network peer_id.
/// Can be triggered by human players (via network) or AI players (via server).
#[derive(Debug, Clone)]
pub struct GameInput {
    pub username: Username,
    pub input: ClientInput,
}

#[derive(Resource, Debug, Default)]
pub struct GameInputQueue {
    pub queue: Vec<GameInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct UnnamedPlayer {
    pub peer_id: PeerId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub struct HumanControl {
    pub peer_id: Option<PeerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub enum AIControl {
    Fermi(FermiControl),
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub enum PlayerControl {
    Human(HumanControl),
    AI(AIControl),
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub struct NamedPlayer {
    pub username: Username,
    pub control: PlayerControl,
}

impl NamedPlayer {
    /// Returns the peer_id if this is a human-controlled player, None otherwise
    pub fn human_peer_id(&self) -> Option<PeerId> {
        match &self.control {
            PlayerControl::Human(human) => human.peer_id,
            PlayerControl::AI(_) => None,
        }
    }

    /// Returns true if this is a human player with the given peer_id
    pub fn is_human_with_peer(&self, peer_id: PeerId) -> bool {
        self.human_peer_id() == Some(peer_id)
    }

    /// Returns true if this is a human player (regardless of connection status)
    pub fn is_human(&self) -> bool {
        matches!(self.control, PlayerControl::Human(_))
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Default)]
pub struct PlayerInfo {
    pub unnamed_players: Vec<UnnamedPlayer>,
    pub named_players: Vec<NamedPlayer>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Default)]
pub struct RoomInfo {
    pub host_peer_id: Option<PeerId>,
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
            .find_map(|named| match &named.control {
                PlayerControl::Human(human_control) if human_control.peer_id == Some(peer_id) => {
                    Some(named.username.clone())
                }
                _ => None,
            })
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Default)]
pub struct Question {
    pub round: Option<u32>,
    pub question: String,
    pub answer: u32,
    pub units: Option<String>,
    pub guesses: HashMap<Username, u32>,
    pub sources: Vec<Source>,
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
    pub delta_this_round: HashMap<Username, i32>,
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
        app.register_component::<RoomInfo>();
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
