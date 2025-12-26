use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Reflect, Default, PartialEq)]
pub enum Inputs {
    #[default]
    Noop,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct Question {
    pub question: String,
    pub answer: i32,
}

impl MapEntities for Inputs {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}

pub struct ChannelSimple;

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<Inputs>::default());

        app.register_component::<Question>();

        app.add_channel::<ChannelSimple>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
    }
}
