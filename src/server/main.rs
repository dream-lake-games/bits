use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::protocol::ProtocolPlugin;
use bits::window::get_window_plugin_with_title;

mod host;
mod server_ai;
mod server_game;
mod server_lobby;
mod server_question;
mod server_simple;
mod server_state;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(get_window_plugin_with_title("Server")))
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        );

    app.add_plugins((
        ProtocolPlugin,
        bits::bits_ui::bits_ui_plugin_fn,
        host::host_plugin_fn,
        server_ai::server_ai_plugin_fn,
        server_game::server_game_plugin_fn,
        server_lobby::server_lobby_plugin_fn,
        server_simple::server_simple_plugin_fn,
        server_state::server_state_plugin_fn,
        server_question::server_question_plugin_fn,
    ));

    app.run();
}
