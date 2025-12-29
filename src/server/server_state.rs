use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ServerState {
    Lobby,
    InGame,
}

pub fn temporary_start_game(mut server_state: ResMut<NextState<ServerState>>) {
    server_state.set(ServerState::InGame);
}

pub fn server_state_plugin_fn(app: &mut App) {
    app.insert_state(ServerState::Lobby);
}
