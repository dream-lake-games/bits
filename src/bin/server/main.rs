use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::window::get_window_plugin_with_title;

mod server_lobby;
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
        bits::bits_ui::bits_ui_plugin_fn,
        server_lobby::server_lobby_plugin_fn,
        server_simple::server_simple_plugin_fn,
        server_state::server_state_plugin_fn,
    ));

    app.run();
}
