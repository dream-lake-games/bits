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

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct BetsActive {
    pub bets_seconds_remaining: Option<f32>,
    pub bets_locked: HashMap<Username, bool>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct BetsRetired;

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct RoundCap {
    round: u32,
    seconds_until_auto_continue: Option<f32>,
    continue_locked: HashMap<Username, bool>,
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
