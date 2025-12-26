use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin, WindowResolution};

pub fn get_window_plugin_with_title(title: &str) -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: title.to_string(),
            resolution: WindowResolution::new(800, 800),
            fit_canvas_to_parent: false,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    }
}
