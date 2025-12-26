use bevy::prelude::*;

pub mod button_simple;

pub use button_simple::{ButtonSimple, ButtonSimpleDrawState};

pub fn bits_ui_plugin_fn(app: &mut App) {
    app.add_plugins(button_simple::button_simple_plugin_fn);
}
