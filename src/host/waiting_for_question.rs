//! WaitingForQuestionScreen - displays "GENERATING QUESTION..." with animated dots.

use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;

/// Marker component for the waiting screen. Spawn this to show the "GENERATING QUESTION" UI.
#[derive(Component, Default)]
#[require(Transform, Visibility)]
pub struct WaitingForQuestionScreen;

fn waiting_text(time: Res<Time>, mut dots: Local<u32>, mut timer: Local<f32>) -> String {
    *timer += time.delta_secs();
    if *timer > 0.4 {
        *timer = 0.0;
        *dots = (*dots + 1) % 4;
    }
    let visible = "*".repeat(*dots as usize);
    let hidden = "_".repeat(3 - *dots as usize);
    format!("GENERATING QUESTION{}{}", visible, hidden)
}

fn handle_waiting_for_question_screen_added(
    trigger: On<Add, WaitingForQuestionScreen>,
    mut commands: Commands,
) {
    commands.entity(trigger.entity).with_children(|parent| {
        parent.spawn((
            Name::new("WaitingText"),
            AnimatedText::new("GENERATING QUESTION___", UVec2::new(704, 64), TEXT_SPEED)
                .with_text_system(waiting_text),
            Transform::default(),
            Visibility::Inherited,
        ));
    });
}

pub fn waiting_for_question_plugin_fn(app: &mut App) {
    app.add_observer(handle_waiting_for_question_screen_added);
}
