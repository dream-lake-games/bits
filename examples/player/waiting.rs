use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::player::{PlayerWaitingScreen, player_waiting_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    commands.spawn((
        Name::new("PlayerWaitingScreen"),
        PlayerWaitingScreen,
    ));
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Player: Waiting"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(player_waiting_plugin_fn)
        .add_systems(Startup, startup)
        .run();
}

