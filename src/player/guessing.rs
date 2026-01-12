//! PlayerGuessingScreen - number input for submitting guesses.
//!
//! Button callbacks use `with_on_release(|mut q: Query<...>| { ... })` which runs as
//! a one-shot system with full ECS access. This lets callbacks directly modify screen
//! state without needing Arc/Mutex for thread-safe shared state.

use crate::LetterAnim;
use crate::bits_ui::AnimButton;
use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;
const BUTTON_SPACING: f32 = 8.0;

/// Data for the player guessing screen.
#[derive(Component, Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
#[require(Transform, Visibility)]
pub struct PlayerGuessingScreen {
    // === DISPLAY DATA ===
    pub question: String,
    pub units: Option<String>,
    pub seconds_remaining: Option<f32>,
    pub already_submitted: Option<u32>,

    // === LOCAL UI STATE ===
    pub pending_guess: String,

    // === OUTPUT ===
    pub submit_requested: bool,
}

impl PlayerGuessingScreen {
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            units: None,
            seconds_remaining: None,
            already_submitted: None,
            pending_guess: String::new(),
            submit_requested: false,
        }
    }

    pub fn with_units(mut self, units: impl Into<String>) -> Self {
        self.units = Some(units.into());
        self
    }

    pub fn with_seconds_remaining(mut self, seconds: f32) -> Self {
        self.seconds_remaining = Some(seconds);
        self
    }

    pub fn with_already_submitted(mut self, value: u32) -> Self {
        self.already_submitted = Some(value);
        self
    }

    fn question_text(&self) -> String {
        match &self.units {
            Some(units) => format!(
                "{} IN {}",
                self.question.to_uppercase(),
                units.to_uppercase()
            ),
            None => self.question.to_uppercase(),
        }
    }

    fn timer_text(&self) -> String {
        let seconds = self.seconds_remaining.unwrap_or(0.0).max(0.0);
        format!("TIME {:.0}", seconds)
    }

    fn display_text(&self) -> String {
        if let Some(submitted) = self.already_submitted {
            format!("SUBMITTED: {}", submitted)
        } else if self.pending_guess.is_empty() {
            "0".to_string()
        } else {
            self.pending_guess.clone()
        }
    }

    fn is_already_submitted(&self) -> bool {
        self.already_submitted.is_some()
    }

    fn is_digit_disabled(&self) -> bool {
        if self.already_submitted.is_some() {
            return true;
        }
        self.pending_guess.len() >= 6
    }

    fn is_clear_delete_disabled(&self) -> bool {
        if self.already_submitted.is_some() {
            return true;
        }
        self.pending_guess.is_empty()
    }

    fn is_submit_disabled(&self) -> bool {
        if self.already_submitted.is_some() {
            return true;
        }
        if self.pending_guess.is_empty() {
            return true;
        }
        if let Ok(value) = self.pending_guess.parse::<u32>() {
            if value == 0 {
                return true;
            }
        }
        false
    }
}

#[derive(Component)]
struct GuessingQuestionText;

#[derive(Component)]
struct GuessingTimerText;

#[derive(Component)]
struct GuessingDisplayText;

#[derive(Component)]
struct GuessingDigitButton;

#[derive(Component)]
struct GuessingClearButton;

#[derive(Component)]
struct GuessingDeleteButton;

#[derive(Component)]
struct GuessingSubmitButton;

fn numpad_button_position(row: usize, col: usize) -> Vec3 {
    let x = (col as f32 - 1.0) * (BUTTON_SIZE as f32 + BUTTON_SPACING);
    let y = (1.5 - row as f32) * (BUTTON_SIZE as f32 + BUTTON_SPACING) - 80.0;
    Vec3::new(x, y, 0.0)
}

fn handle_digit(digit: char, screen_q: &mut Query<&mut PlayerGuessingScreen>) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    if screen.is_already_submitted() {
        return;
    }
    let new_value = format!("{}{}", screen.pending_guess, digit);
    if let Ok(parsed) = new_value.parse::<u32>() {
        let as_string = parsed.to_string();
        if as_string.len() <= 6 {
            screen.pending_guess = as_string;
        }
    }
}

fn handle_player_guessing_screen_added(
    trigger: On<Add, PlayerGuessingScreen>,
    screen_q: Query<&PlayerGuessingScreen>,
    mut commands: Commands,
) {
    let screen = screen_q
        .get(trigger.entity)
        .expect("PlayerGuessingScreen should exist");

    commands.entity(trigger.entity).with_children(|parent| {
        // Question text at top
        parent.spawn((
            Name::new("Question"),
            GuessingQuestionText,
            AnimatedText::new(&screen.question_text(), UVec2::new(768, 128), TEXT_SPEED),
            Transform::from_xyz(0.0, 280.0, 0.0),
            Visibility::Inherited,
        ));

        // Timer
        parent.spawn((
            Name::new("Timer"),
            GuessingTimerText,
            AnimatedText::new(&screen.timer_text(), UVec2::new(256, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, 180.0, 0.0),
            Visibility::Inherited,
        ));

        // Number display
        parent.spawn((
            Name::new("Display"),
            GuessingDisplayText,
            AnimatedText::new(&screen.display_text(), UVec2::new(256, 64), TEXT_SPEED),
            Transform::from_xyz(0.0, 116.0, 0.0),
            Visibility::Inherited,
        ));

        // Numpad buttons: 1-9 in 3x3 grid
        let digits = [
            ('1', 0, 0, LetterAnim::One),
            ('2', 0, 1, LetterAnim::Two),
            ('3', 0, 2, LetterAnim::Three),
            ('4', 1, 0, LetterAnim::Four),
            ('5', 1, 1, LetterAnim::Five),
            ('6', 1, 2, LetterAnim::Six),
            ('7', 2, 0, LetterAnim::Seven),
            ('8', 2, 1, LetterAnim::Eight),
            ('9', 2, 2, LetterAnim::Nine),
        ];

        for (digit, row, col, letter) in digits {
            parent.spawn((
                Name::new(format!("Button_{}", digit)),
                GuessingDigitButton,
                AnimButton::new(letter).with_on_release(
                    move |mut screen_q: Query<&mut PlayerGuessingScreen>| {
                        handle_digit(digit, &mut screen_q);
                    },
                ),
                Transform::from_translation(numpad_button_position(row, col)),
                Visibility::Inherited,
            ));
        }

        // Bottom row: CLR, 0, DEL
        parent.spawn((
            Name::new("Button_CLR"),
            GuessingClearButton,
            AnimButton::new(LetterAnim::Clear).with_on_release(
                |mut screen_q: Query<&mut PlayerGuessingScreen>| {
                    let Ok(mut screen) = screen_q.single_mut() else {
                        return;
                    };
                    if !screen.is_already_submitted() {
                        screen.pending_guess.clear();
                    }
                },
            ),
            Transform::from_translation(numpad_button_position(3, 0)),
            Visibility::Inherited,
        ));

        parent.spawn((
            Name::new("Button_0"),
            GuessingDigitButton,
            AnimButton::new(LetterAnim::Zero).with_on_release(
                |mut screen_q: Query<&mut PlayerGuessingScreen>| {
                    handle_digit('0', &mut screen_q);
                },
            ),
            Transform::from_translation(numpad_button_position(3, 1)),
            Visibility::Inherited,
        ));

        parent.spawn((
            Name::new("Button_DEL"),
            GuessingDeleteButton,
            AnimButton::new(LetterAnim::Back).with_on_release(
                |mut screen_q: Query<&mut PlayerGuessingScreen>| {
                    let Ok(mut screen) = screen_q.single_mut() else {
                        return;
                    };
                    if !screen.is_already_submitted() {
                        screen.pending_guess.pop();
                    }
                },
            ),
            Transform::from_translation(numpad_button_position(3, 2)),
            Visibility::Inherited,
        ));

        // Submit button
        parent.spawn((
            Name::new("Button_Submit"),
            GuessingSubmitButton,
            AnimButton::new(LetterAnim::Plus).with_on_release(
                |mut screen_q: Query<&mut PlayerGuessingScreen>| {
                    let Ok(mut screen) = screen_q.single_mut() else {
                        return;
                    };
                    if !screen.is_submit_disabled() {
                        screen.submit_requested = true;
                    }
                },
            ),
            Transform::from_translation(numpad_button_position(4, 1)),
            Visibility::Inherited,
        ));
    });
}

fn update_guessing_screen_text(
    screen_q: Query<(&PlayerGuessingScreen, &Children)>,
    mut question_q: Query<&mut AnimatedText, With<GuessingQuestionText>>,
    mut timer_q: Query<&mut AnimatedText, (With<GuessingTimerText>, Without<GuessingQuestionText>)>,
    mut display_q: Query<
        &mut AnimatedText,
        (
            With<GuessingDisplayText>,
            Without<GuessingQuestionText>,
            Without<GuessingTimerText>,
        ),
    >,
) {
    for (screen, children) in &screen_q {
        let display_text = screen.display_text();
        for child in children.iter() {
            if let Ok(mut text) = question_q.get_mut(child) {
                text.text_this_frame = screen.question_text();
            }
            if let Ok(mut text) = timer_q.get_mut(child) {
                text.text_this_frame = screen.timer_text();
            }
            if let Ok(mut text) = display_q.get_mut(child) {
                text.text_this_frame = display_text.clone();
            }
        }
    }
}

fn update_guessing_button_disabled(
    screen_q: Query<(&PlayerGuessingScreen, &Children)>,
    mut digit_q: Query<&mut AnimButton, With<GuessingDigitButton>>,
    mut clear_q: Query<&mut AnimButton, (With<GuessingClearButton>, Without<GuessingDigitButton>)>,
    mut delete_q: Query<
        &mut AnimButton,
        (
            With<GuessingDeleteButton>,
            Without<GuessingDigitButton>,
            Without<GuessingClearButton>,
        ),
    >,
    mut submit_q: Query<
        &mut AnimButton,
        (
            With<GuessingSubmitButton>,
            Without<GuessingDigitButton>,
            Without<GuessingClearButton>,
            Without<GuessingDeleteButton>,
        ),
    >,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            if let Ok(mut button) = digit_q.get_mut(child) {
                button.is_disabled = screen.is_digit_disabled();
            }
            if let Ok(mut button) = clear_q.get_mut(child) {
                button.is_disabled = screen.is_clear_delete_disabled();
            }
            if let Ok(mut button) = delete_q.get_mut(child) {
                button.is_disabled = screen.is_clear_delete_disabled();
            }
            if let Ok(mut button) = submit_q.get_mut(child) {
                button.is_disabled = screen.is_submit_disabled();
            }
        }
    }
}

pub fn player_guessing_plugin_fn(app: &mut App) {
    app.register_type::<PlayerGuessingScreen>();
    app.add_observer(handle_player_guessing_screen_added);
    app.add_systems(
        Update,
        (update_guessing_button_disabled, update_guessing_screen_text).chain(),
    );
}
