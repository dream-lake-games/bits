//! PlayerReviewingScreen - round summary with continue button.

use crate::LetterAnim;
use crate::bits_ui::AnimButton;
use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;

/// Data for the player reviewing screen.
#[derive(Component, Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
#[require(Transform, Visibility)]
pub struct PlayerReviewingScreen {
    // === DISPLAY DATA ===
    pub delta_this_round: i32,
    pub total_score: u32,
    pub already_voted: bool,
    pub seconds_until_auto: Option<f32>,

    // === OUTPUT ===
    pub continue_requested: bool,
}

impl PlayerReviewingScreen {
    pub fn new(delta: i32) -> Self {
        Self {
            delta_this_round: delta,
            total_score: 0,
            already_voted: false,
            seconds_until_auto: None,
            continue_requested: false,
        }
    }

    pub fn with_total_score(mut self, score: u32) -> Self {
        self.total_score = score;
        self
    }

    pub fn with_already_voted(mut self, voted: bool) -> Self {
        self.already_voted = voted;
        self
    }

    pub fn with_seconds_until_auto(mut self, seconds: f32) -> Self {
        self.seconds_until_auto = Some(seconds);
        self
    }

    fn header_text(&self) -> String {
        "ROUND SUMMARY".to_string()
    }

    fn delta_text(&self) -> String {
        if self.delta_this_round >= 0 {
            format!("+{}", self.delta_this_round)
        } else {
            format!("{}", self.delta_this_round)
        }
    }

    fn score_text(&self) -> String {
        format!("TOTAL SCORE: {}", self.total_score)
    }

    fn timer_text(&self) -> String {
        let seconds = self.seconds_until_auto.unwrap_or(0.0).max(0.0);
        format!("AUTO IN {:.0}", seconds)
    }

    fn status_text(&self) -> String {
        if self.already_voted {
            "WAITING FOR OTHERS".to_string()
        } else {
            "".to_string()
        }
    }
}

fn handle_player_reviewing_screen_added(
    trigger: On<Add, PlayerReviewingScreen>,
    screen_q: Query<&PlayerReviewingScreen>,
    mut commands: Commands,
) {
    let screen = screen_q
        .get(trigger.entity)
        .expect("PlayerReviewingScreen should exist")
        .clone();

    commands.entity(trigger.entity).with_children(|parent| {
        // Header
        parent.spawn((
            Name::new("Header"),
            ReviewingHeaderText,
            AnimatedText::new(&screen.header_text(), UVec2::new(512, 64), TEXT_SPEED),
            Transform::from_xyz(0.0, 200.0, 0.0),
            Visibility::Inherited,
        ));

        // Delta (big, centered)
        parent.spawn((
            Name::new("Delta"),
            ReviewingDeltaText,
            AnimatedText::new(&screen.delta_text(), UVec2::new(256, 128), TEXT_SPEED)
                .with_size(AnimatedTextSize::Large),
            Transform::from_xyz(0.0, 50.0, 0.0),
            Visibility::Inherited,
        ));

        // Total score
        parent.spawn((
            Name::new("Score"),
            ReviewingScoreText,
            AnimatedText::new(&screen.score_text(), UVec2::new(320, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, -30.0, 0.0),
            Visibility::Inherited,
        ));

        // Timer
        parent.spawn((
            Name::new("Timer"),
            ReviewingTimerText,
            AnimatedText::new(&screen.timer_text(), UVec2::new(256, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, -70.0, 0.0),
            Visibility::Inherited,
        ));

        // Status text (shown when already voted)
        parent.spawn((
            Name::new("Status"),
            ReviewingStatusText,
            AnimatedText::new(&screen.status_text(), UVec2::new(384, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, -150.0, 0.0),
            Visibility::Inherited,
        ));

        // Continue button with label
        parent.spawn((
            Name::new("ContinueLabel"),
            AnimatedText::new("CONTINUE", UVec2::new(160, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(-56.0, -250.0, 0.0),
            Visibility::Inherited,
        ));

        parent.spawn((
            Name::new("ContinueButton"),
            ReviewingContinueButton,
            AnimButton::new(LetterAnim::Plus).with_on_release(
                |mut screen_q: Query<&mut PlayerReviewingScreen>| {
                    if let Ok(mut screen) = screen_q.single_mut() {
                        if !screen.already_voted {
                            screen.continue_requested = true;
                        }
                    }
                },
            ),
            Transform::from_xyz(56.0, -250.0, 0.0),
            Visibility::Inherited,
        ));
    });
}

#[derive(Component)]
struct ReviewingHeaderText;

#[derive(Component)]
struct ReviewingDeltaText;

#[derive(Component)]
struct ReviewingTimerText;

#[derive(Component)]
struct ReviewingScoreText;

#[derive(Component)]
struct ReviewingStatusText;

#[derive(Component)]
struct ReviewingContinueButton;

fn update_reviewing_screen_text(
    screen_q: Query<(&PlayerReviewingScreen, &Children)>,
    mut header_q: Query<&mut AnimatedText, With<ReviewingHeaderText>>,
    mut delta_q: Query<&mut AnimatedText, (With<ReviewingDeltaText>, Without<ReviewingHeaderText>)>,
    mut score_q: Query<
        &mut AnimatedText,
        (
            With<ReviewingScoreText>,
            Without<ReviewingHeaderText>,
            Without<ReviewingDeltaText>,
        ),
    >,
    mut timer_q: Query<
        &mut AnimatedText,
        (
            With<ReviewingTimerText>,
            Without<ReviewingHeaderText>,
            Without<ReviewingDeltaText>,
            Without<ReviewingScoreText>,
        ),
    >,
    mut status_q: Query<
        &mut AnimatedText,
        (
            With<ReviewingStatusText>,
            Without<ReviewingHeaderText>,
            Without<ReviewingDeltaText>,
            Without<ReviewingScoreText>,
            Without<ReviewingTimerText>,
        ),
    >,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            if let Ok(mut text) = header_q.get_mut(child) {
                text.text_this_frame = screen.header_text();
            }
            if let Ok(mut text) = delta_q.get_mut(child) {
                text.text_this_frame = screen.delta_text();
            }
            if let Ok(mut text) = score_q.get_mut(child) {
                text.text_this_frame = screen.score_text();
            }
            if let Ok(mut text) = timer_q.get_mut(child) {
                text.text_this_frame = screen.timer_text();
            }
            if let Ok(mut text) = status_q.get_mut(child) {
                text.text_this_frame = screen.status_text();
            }
        }
    }
}

fn update_reviewing_button_disabled(
    screen_q: Query<(&PlayerReviewingScreen, &Children)>,
    mut button_q: Query<&mut AnimButton, With<ReviewingContinueButton>>,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            if let Ok(mut button) = button_q.get_mut(child) {
                button.is_disabled = screen.already_voted;
            }
        }
    }
}

pub fn player_reviewing_plugin_fn(app: &mut App) {
    app.register_type::<PlayerReviewingScreen>();
    app.add_observer(handle_player_reviewing_screen_added);
    app.add_systems(
        Update,
        (
            update_reviewing_button_disabled,
            update_reviewing_screen_text,
        )
            .chain(),
    );
}
