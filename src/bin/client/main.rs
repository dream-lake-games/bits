use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::window::get_window_plugin_with_title;

mod client_simple;

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

    app.add_plugins(client_simple::client_simple_plugin_fn);

    app.run();
}
