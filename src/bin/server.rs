use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, server_loop)
        .run();
}

fn setup() {}

fn server_loop() {}
