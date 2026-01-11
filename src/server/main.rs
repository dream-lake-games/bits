use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::winit::WinitPlugin;
use bits::protocol::ProtocolPlugin;

mod server_ai;
mod server_game;
mod server_lobby;
mod server_question;
mod server_simple;
mod server_state;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.build().disable::<WinitPlugin>(),
        ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0)),
    ));

    app.add_plugins((
        ProtocolPlugin,
        server_ai::server_ai_plugin_fn,
        server_game::server_game_plugin_fn,
        server_lobby::server_lobby_plugin_fn,
        server_simple::server_simple_plugin_fn,
        server_state::server_state_plugin_fn,
        server_question::server_question_plugin_fn,
    ));

    info!("Server starting headless");
    app.run();
}
