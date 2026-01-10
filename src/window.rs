use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin, WindowResolution};

pub const WINDOW_SIZE: u32 = 800;

pub fn get_window_plugin_with_title(title: &str) -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: title.to_string(),
            resolution: WindowResolution::new(WINDOW_SIZE, WINDOW_SIZE),
            fit_canvas_to_parent: false,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    }
}
