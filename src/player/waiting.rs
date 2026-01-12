//! PlayerWaitingScreen - displays "WAITING FOR QUESTION..." with animated dots.

use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;

/// Marker component for the player waiting screen.
/// Spawn this to show the "WAITING FOR QUESTION" UI.
#[derive(Component, Default)]
#[require(Transform, Visibility)]
pub struct PlayerWaitingScreen;

fn waiting_text(time: Res<Time>, mut dots: Local<u32>, mut timer: Local<f32>) -> String {
    *timer += time.delta_secs();
    if *timer > 0.4 {
        *timer = 0.0;
        *dots = (*dots + 1) % 4;
    }
    let visible = "*".repeat(*dots as usize);
    let hidden = "_".repeat(3 - *dots as usize);
    format!("WAITING FOR QUESTION{}{}", visible, hidden)
}

fn handle_player_waiting_screen_added(
    trigger: On<Add, PlayerWaitingScreen>,
    mut commands: Commands,
) {
    commands.entity(trigger.entity).with_children(|parent| {
        parent.spawn((
            Name::new("WaitingText"),
            AnimatedText::new("WAITING FOR QUESTION___", UVec2::new(736, 64), TEXT_SPEED)
                .with_text_system(waiting_text),
            Transform::default(),
            Visibility::Inherited,
        ));
    });
}

pub fn player_waiting_plugin_fn(app: &mut App) {
    app.add_observer(handle_player_waiting_screen_added);
}

