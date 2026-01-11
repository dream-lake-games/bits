use bevy::prelude::*;

pub mod anim;
pub mod assemble;
pub mod button_simple;
pub mod colors;
pub mod flex_simple;
pub mod spacer;
pub mod text;
pub mod text_simple;

pub use anim::{Anim, AnimConfig, AnimMan, AnimNextIndex, AnimPlugin, AnimVariant};
pub use assemble::Assemble;
pub use button_simple::{ButtonSimple, ButtonSimpleDrawState};
pub use flex_simple::FlexSimple;
pub use spacer::Spacer;
pub use text::{AnimatedText, BUTTON_SIZE, Button, LETTER_SIZE};
pub use text_simple::TextSimple;

pub fn bits_ui_plugin_fn(app: &mut App) {
    app.add_plugins((
        AnimPlugin { default_fps: 4.0 },
        assemble::assemble_simple_plugin_fn,
        button_simple::button_simple_plugin_fn,
        flex_simple::flex_simple_plugin_fn,
        text_simple::text_simple_plugin_fn,
        text::text_plugin_fn,
        text::button_plugin_fn,
    ));
}
