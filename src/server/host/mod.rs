mod host_game;
mod host_state;

use bevy::prelude::*;

pub fn host_plugin_fn(app: &mut App) {
    app.add_plugins((
        host_game::host_game_plugin_fn,
        host_state::host_state_plugin_fn,
    ));
}
