use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bits::protocol::ProtocolPlugin;
use clap::Parser;

mod server_ai;
mod server_game;
mod server_lobby;
mod server_question;
mod server_simple;
mod server_state;

#[derive(Parser, Debug, Resource, Clone)]
#[command(name = "server")]
#[command(about = "Bits game server", long_about = None)]
struct Args {
    #[arg(long)]
    room_code: String,

    #[arg(long, default_value = "http://lobby:8080")]
    lobby_url: String,
}

fn main() {
    let args = Args::parse();

    let mut app = App::new();

    app.insert_resource(args);

    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        ))),
        LogPlugin::default(),
        StatesPlugin,
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
