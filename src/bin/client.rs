use bevy::prelude::*;
use bevy::window::{Window, WindowResolution};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(800, 800),
                fit_canvas_to_parent: false,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, game_loop)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn game_loop() {}
