use bevy::prelude::*;

use crate::bg::{BgMarker, bg_plugin_fn};

pub mod anim;
pub mod assemble;
pub mod button_simple;
pub mod colors;
pub mod flex_simple;
pub mod sentence_list;
pub mod spacer;
pub mod text;
pub mod text_simple;

pub use anim::{Anim, AnimConfig, AnimMan, AnimNextIndex, AnimPlugin, AnimVariant};
pub use assemble::Assemble;
pub use button_simple::{ButtonSimple, ButtonSimpleDrawState};
pub use flex_simple::FlexSimple;
pub use sentence_list::SentenceList;
pub use spacer::Spacer;
pub use text::{AnimatedText, BUTTON_SIZE, Button, LETTER_SIZE};
pub use text_simple::TextSimple;

fn spawn_bg(mut commands: Commands) {
    commands.spawn((
        Name::new("Background"),
        BgMarker::default(),
        Transform::default(),
        Visibility::Inherited,
    ));
}

fn spawn_score_placeholder(mut commands: Commands) {
    commands.spawn((
        Name::new("ScorePlaceholder"),
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.1),
            custom_size: Some(Vec2::new(800.0, 160.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -320.0, 0.0),
    ));
}

pub fn bits_ui_plugin_fn(app: &mut App) {
    app.add_plugins((
        AnimPlugin { default_fps: 4.0 },
        assemble::assemble_simple_plugin_fn,
        button_simple::button_simple_plugin_fn,
        flex_simple::flex_simple_plugin_fn,
        sentence_list::sentence_list_plugin_fn,
        text_simple::text_simple_plugin_fn,
        text::text_plugin_fn,
        text::button_plugin_fn,
        bg_plugin_fn,
    ));
    app.add_systems(Startup, (spawn_bg, spawn_score_placeholder));
}
