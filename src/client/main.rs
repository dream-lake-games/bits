use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::protocol::ProtocolPlugin;
use bits::window::get_window_plugin_with_title;

mod client_game;
mod client_game_host;
mod client_lobby;
mod client_simple;
mod client_state;
mod host;
mod player;

fn main() {
    let mut app = App::new();

    let client_name = std::env::var("CLIENT_NAME").unwrap_or_else(|_| "Unknown".to_string());
    let window_title = format!("Client {}", client_name);

    app.add_plugins(DefaultPlugins.set(get_window_plugin_with_title(&window_title)))
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        );

    app.add_plugins((
        ProtocolPlugin,
        bits_ui_plugin_fn,
        client_game::client_game_plugin_fn,
        client_game_host::client_game_host_plugin_fn,
        client_lobby::client_lobby_plugin_fn,
        client_simple::client_simple_plugin_fn,
        client_state::client_state_plugin_fn,
        host::host_lobby_plugin_fn,
        player::player_lobby_plugin_fn,
    ));

    app.run();
}
