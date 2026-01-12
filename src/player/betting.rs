//! PlayerBettingScreen - interactive betting controls for player phones.

use crate::LetterAnim;
use crate::bits_ui::AnimButton;
use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;

/// Display data for a single guess that can be bet on.
#[derive(Clone, Default, Reflect)]
pub struct BettingGuessDisplay {
    pub value: u32,
    pub owners: Vec<String>,
    pub my_free_bets: u32,
    pub my_paid_bets: u32,
}

/// Output when a bet button is pressed.
#[derive(Clone, Reflect, Debug)]
pub struct PendingBet {
    pub guess: u32,
    pub num_free: u32,
    pub num_paid: u32,
}

/// Data for the player betting screen.
#[derive(Component, Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
#[require(Transform, Visibility)]
pub struct PlayerBettingScreen {
    // === DISPLAY DATA ===
    pub question: String,
    pub units: Option<String>,
    pub seconds_remaining: Option<f32>,
    pub player_score: u32,
    pub is_locked: bool,
    pub guesses: Vec<BettingGuessDisplay>,

    // === OUTPUT ===
    pub pending_bet: Option<PendingBet>,
    pub lock_requested: bool,
}

impl PlayerBettingScreen {
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            units: None,
            seconds_remaining: None,
            player_score: 0,
            is_locked: false,
            guesses: Vec::new(),
            pending_bet: None,
            lock_requested: false,
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

    pub fn with_player_score(mut self, score: u32) -> Self {
        self.player_score = score;
        self
    }

    pub fn with_is_locked(mut self, locked: bool) -> Self {
        self.is_locked = locked;
        self
    }

    pub fn with_guesses(mut self, guesses: Vec<BettingGuessDisplay>) -> Self {
        self.guesses = guesses;
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

    fn status_text(&self) -> String {
        if self.is_locked {
            "LOCKED!".to_string()
        } else {
            format!("SCORE: {}", self.player_score)
        }
    }

    fn total_free_bets(&self) -> u32 {
        self.guesses.iter().map(|g| g.my_free_bets).sum()
    }

    fn total_paid_bets(&self) -> u32 {
        self.guesses.iter().map(|g| g.my_paid_bets).sum()
    }
}

/// Called by button callbacks to modify bets. Reads current bet values from screen.guesses[index]
/// and sets pending_bet with the delta applied. The index is captured in each button's closure
/// at spawn time, while current values are read fresh from the screen each click.
fn handle_bet_change(
    index: usize,
    free_delta: i32,
    paid_delta: i32,
    screen_q: &mut Query<&mut PlayerBettingScreen>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    if screen.is_locked {
        return;
    }
    let Some(guess) = screen.guesses.get(index) else {
        return;
    };
    let guess_value = guess.value;
    let new_free = (guess.my_free_bets as i32 + free_delta).max(0) as u32;
    let new_paid = (guess.my_paid_bets as i32 + paid_delta).max(0) as u32;

    screen.pending_bet = Some(PendingBet {
        guess: guess_value,
        num_free: new_free,
        num_paid: new_paid,
    });
}

/// Spawns all UI children when a PlayerBettingScreen is added. Button callbacks are closures
/// that capture the guess index and call handle_bet_change with ECS query access. The guesses
/// must be populated before spawning - see spawn_betting_screen_when_ready in client_game.rs.
fn handle_player_betting_screen_added(
    trigger: On<Add, PlayerBettingScreen>,
    screen_q: Query<&PlayerBettingScreen>,
    mut commands: Commands,
) {
    let screen = screen_q
        .get(trigger.entity)
        .expect("PlayerBettingScreen should exist")
        .clone();

    commands.entity(trigger.entity).with_children(|parent| {
        // Question text at top
        parent.spawn((
            Name::new("Question"),
            BettingQuestionText,
            AnimatedText::new(&screen.question_text(), UVec2::new(768, 128), TEXT_SPEED),
            Transform::from_xyz(0.0, 300.0, 0.0),
            Visibility::Inherited,
        ));

        // Timer
        parent.spawn((
            Name::new("Timer"),
            BettingTimerText,
            AnimatedText::new(&screen.timer_text(), UVec2::new(256, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, 200.0, 0.0),
            Visibility::Inherited,
        ));

        // Status (score or locked)
        parent.spawn((
            Name::new("Status"),
            BettingStatusText,
            AnimatedText::new(&screen.status_text(), UVec2::new(256, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, 160.0, 0.0),
            Visibility::Inherited,
        ));

        // Lock button at bottom with label
        parent.spawn((
            Name::new("LockLabel"),
            AnimatedText::new("LOCK BETS", UVec2::new(160, 32), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(-56.0, -280.0, 0.0),
            Visibility::Inherited,
        ));

        parent.spawn((
            Name::new("LockButton"),
            BettingLockButton,
            AnimButton::new(LetterAnim::Exclaim).with_on_release(
                |mut screen_q: Query<&mut PlayerBettingScreen>| {
                    if let Ok(mut screen) = screen_q.single_mut() {
                        if !screen.is_locked {
                            screen.lock_requested = true;
                        }
                    }
                },
            ),
            Transform::from_xyz(56.0, -280.0, 0.0),
            Visibility::Inherited,
        ));

        // Spawn guess boxes
        let total = screen.guesses.len();
        let free_row_y = -30.0;
        let paid_row_y = -130.0;
        for (index, guess) in screen.guesses.iter().enumerate() {
            let x = calculate_guess_x(index, total);

            // Text display for this guess (owners + value)
            parent.spawn((
                Name::new(format!("GuessText_{}", index)),
                BettingGuessText { index },
                SentenceList::new(guess_box_sentences(guess))
                    .with_text_speed(TEXT_SPEED)
                    .with_size(AnimatedTextSize::Small),
                Transform::from_xyz(x, 60.0, 0.0),
                Visibility::Inherited,
            ));

            // FREE label
            parent.spawn((
                Name::new(format!("FreeLabel_{}", index)),
                BettingGuessBox,
                AnimatedText::new("FREE", UVec2::new(64, 16), TEXT_SPEED)
                    .with_size(AnimatedTextSize::Small),
                Transform::from_xyz(x, free_row_y + 40.0, 0.0),
                Visibility::Inherited,
            ));

            // Free bet: - count +
            parent.spawn((
                Name::new(format!("FreeMinus_{}", index)),
                BettingGuessBox,
                BettingFreeMinusButton { index },
                AnimButton::new(LetterAnim::Minus).with_on_release(
                    move |mut screen_q: Query<&mut PlayerBettingScreen>| {
                        handle_bet_change(index, -1, 0, &mut screen_q);
                    },
                ),
                Transform::from_xyz(x - 48.0, free_row_y, 0.0),
                Visibility::Inherited,
            ));

            parent.spawn((
                Name::new(format!("FreeCount_{}", index)),
                BettingGuessBox,
                BettingFreeCountText { index },
                AnimatedText::new(
                    &guess.my_free_bets.to_string(),
                    UVec2::new(32, 32),
                    TEXT_SPEED,
                )
                .with_size(AnimatedTextSize::Small),
                Transform::from_xyz(x, free_row_y, 0.0),
                Visibility::Inherited,
            ));

            parent.spawn((
                Name::new(format!("FreePlus_{}", index)),
                BettingGuessBox,
                BettingFreePlusButton,
                AnimButton::new(LetterAnim::Plus).with_on_release(
                    move |mut screen_q: Query<&mut PlayerBettingScreen>| {
                        handle_bet_change(index, 1, 0, &mut screen_q);
                    },
                ),
                Transform::from_xyz(x + 48.0, free_row_y, 0.0),
                Visibility::Inherited,
            ));

            // PAID label
            parent.spawn((
                Name::new(format!("PaidLabel_{}", index)),
                BettingGuessBox,
                AnimatedText::new("PAID", UVec2::new(64, 16), TEXT_SPEED)
                    .with_size(AnimatedTextSize::Small),
                Transform::from_xyz(x, paid_row_y + 40.0, 0.0),
                Visibility::Inherited,
            ));

            // Paid bet: - count +
            parent.spawn((
                Name::new(format!("PaidMinus_{}", index)),
                BettingGuessBox,
                BettingPaidMinusButton { index },
                AnimButton::new(LetterAnim::Minus).with_on_release(
                    move |mut screen_q: Query<&mut PlayerBettingScreen>| {
                        handle_bet_change(index, 0, -1, &mut screen_q);
                    },
                ),
                Transform::from_xyz(x - 48.0, paid_row_y, 0.0),
                Visibility::Inherited,
            ));

            parent.spawn((
                Name::new(format!("PaidCount_{}", index)),
                BettingGuessBox,
                BettingPaidCountText { index },
                AnimatedText::new(
                    &guess.my_paid_bets.to_string(),
                    UVec2::new(32, 32),
                    TEXT_SPEED,
                )
                .with_size(AnimatedTextSize::Small),
                Transform::from_xyz(x, paid_row_y, 0.0),
                Visibility::Inherited,
            ));

            parent.spawn((
                Name::new(format!("PaidPlus_{}", index)),
                BettingGuessBox,
                BettingPaidPlusButton { index },
                AnimButton::new(LetterAnim::Plus).with_on_release(
                    move |mut screen_q: Query<&mut PlayerBettingScreen>| {
                        handle_bet_change(index, 0, 1, &mut screen_q);
                    },
                ),
                Transform::from_xyz(x + 48.0, paid_row_y, 0.0),
                Visibility::Inherited,
            ));
        }
    });
}

#[derive(Component)]
struct BettingQuestionText;

#[derive(Component)]
struct BettingTimerText;

#[derive(Component)]
struct BettingStatusText;

#[derive(Component)]
struct BettingGuessBox;

#[derive(Component)]
struct BettingFreeCountText {
    index: usize,
}

#[derive(Component)]
struct BettingPaidCountText {
    index: usize,
}

#[derive(Component)]
struct BettingGuessText {
    index: usize,
}

#[derive(Component)]
struct BettingFreeMinusButton {
    index: usize,
}

#[derive(Component)]
struct BettingFreePlusButton;

#[derive(Component)]
struct BettingPaidMinusButton {
    index: usize,
}

#[derive(Component)]
struct BettingPaidPlusButton {
    index: usize,
}

#[derive(Component)]
struct BettingLockButton;

fn calculate_guess_x(index: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let slot_width = 800.0 / total as f32;
    -400.0 + (index as f32 + 0.5) * slot_width
}

fn guess_box_sentences(guess: &BettingGuessDisplay) -> Vec<String> {
    let mut sentences = Vec::new();

    // Line 1: Owners or LOWBALL
    if guess.value == 0 {
        sentences.push("LOWBALL".to_string());
    } else {
        let mut owners = guess.owners.clone();
        owners.sort();
        sentences.push(owners.join(", "));
    }

    // Line 2: The guess value
    sentences.push(guess.value.to_string());

    sentences
}

fn update_betting_screen_text(
    screen_q: Query<(&PlayerBettingScreen, &Children)>,
    mut question_q: Query<&mut AnimatedText, With<BettingQuestionText>>,
    mut timer_q: Query<&mut AnimatedText, (With<BettingTimerText>, Without<BettingQuestionText>)>,
    mut status_q: Query<
        &mut AnimatedText,
        (
            With<BettingStatusText>,
            Without<BettingQuestionText>,
            Without<BettingTimerText>,
        ),
    >,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            if let Ok(mut text) = question_q.get_mut(child) {
                text.text_this_frame = screen.question_text();
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

fn update_betting_guess_boxes(
    screen_q: Query<(&PlayerBettingScreen, &Children)>,
    guess_text_q: Query<&BettingGuessText>,
    free_count_q: Query<&BettingFreeCountText>,
    paid_count_q: Query<&BettingPaidCountText>,
    mut sentence_list_q: Query<&mut SentenceList>,
    mut animated_text_q: Query<&mut AnimatedText>,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            // Update sentence lists (owners + value)
            if let Ok(guess_text) = guess_text_q.get(child) {
                if let Some(guess) = screen.guesses.get(guess_text.index) {
                    if let Ok(mut sentence_list) = sentence_list_q.get_mut(child) {
                        sentence_list.sentences = guess_box_sentences(guess);
                    }
                }
            }

            // Update free count text
            if let Ok(free_count) = free_count_q.get(child) {
                if let Some(guess) = screen.guesses.get(free_count.index) {
                    if let Ok(mut text) = animated_text_q.get_mut(child) {
                        text.text_this_frame = guess.my_free_bets.to_string();
                    }
                }
            }

            // Update paid count text
            if let Ok(paid_count) = paid_count_q.get(child) {
                if let Some(guess) = screen.guesses.get(paid_count.index) {
                    if let Ok(mut text) = animated_text_q.get_mut(child) {
                        text.text_this_frame = guess.my_paid_bets.to_string();
                    }
                }
            }
        }
    }
}

fn update_betting_button_disabled(
    screen_q: Query<(&PlayerBettingScreen, &Children)>,
    mut lock_button_q: Query<&mut AnimButton, With<BettingLockButton>>,
    mut free_minus_q: Query<(&BettingFreeMinusButton, &mut AnimButton), Without<BettingLockButton>>,
    mut free_plus_q: Query<
        &mut AnimButton,
        (
            With<BettingFreePlusButton>,
            Without<BettingLockButton>,
            Without<BettingFreeMinusButton>,
        ),
    >,
    mut paid_minus_q: Query<
        (&BettingPaidMinusButton, &mut AnimButton),
        (
            Without<BettingLockButton>,
            Without<BettingFreeMinusButton>,
            Without<BettingFreePlusButton>,
        ),
    >,
    mut paid_plus_q: Query<
        (&BettingPaidPlusButton, &mut AnimButton),
        (
            Without<BettingLockButton>,
            Without<BettingFreeMinusButton>,
            Without<BettingFreePlusButton>,
            Without<BettingPaidMinusButton>,
        ),
    >,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            if let Ok(mut button) = lock_button_q.get_mut(child) {
                button.is_disabled = screen.is_locked;
            }

            if let Ok((btn, mut button)) = free_minus_q.get_mut(child) {
                button.is_disabled = if screen.is_locked {
                    true
                } else {
                    screen
                        .guesses
                        .get(btn.index)
                        .map(|g| g.my_free_bets == 0)
                        .unwrap_or(true)
                };
            }

            if let Ok(mut button) = free_plus_q.get_mut(child) {
                button.is_disabled = if screen.is_locked {
                    true
                } else {
                    screen.total_free_bets() >= 2
                };
            }

            if let Ok((btn, mut button)) = paid_minus_q.get_mut(child) {
                button.is_disabled = if screen.is_locked {
                    true
                } else {
                    screen
                        .guesses
                        .get(btn.index)
                        .map(|g| g.my_paid_bets == 0)
                        .unwrap_or(true)
                };
            }

            if let Ok((btn, mut button)) = paid_plus_q.get_mut(child) {
                button.is_disabled = if screen.is_locked {
                    true
                } else {
                    let has_free = screen
                        .guesses
                        .get(btn.index)
                        .map(|g| g.my_free_bets > 0)
                        .unwrap_or(false);
                    if !has_free {
                        true
                    } else {
                        screen.total_paid_bets() >= screen.player_score
                    }
                };
            }
        }
    }
}

pub fn player_betting_plugin_fn(app: &mut App) {
    app.register_type::<BettingGuessDisplay>();
    app.register_type::<PendingBet>();
    app.register_type::<PlayerBettingScreen>();
    app.add_observer(handle_player_betting_screen_added);
    app.add_systems(
        Update,
        (
            update_betting_button_disabled,
            update_betting_screen_text,
            update_betting_guess_boxes,
        )
            .chain(),
    );
}
