//! BettingScreen - displays the question, timer, lock status, and guesses/bets.

use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;

/// A single bet placed on a guess.
#[derive(Clone, Default, Reflect)]
pub struct BetEntry {
    pub player: String,
    pub amount: u32,
}

/// A guess value with who guessed it and who bet on it.
#[derive(Clone, Default, Reflect)]
pub struct GuessEntry {
    pub value: u32,
    pub guessers: Vec<String>,
    pub bets: Vec<BetEntry>,
}

/// Data for the betting phase screen.
#[derive(Component, Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
#[require(Transform, Visibility)]
pub struct BettingScreen {
    pub question: String,
    pub units: Option<String>,
    pub seconds_remaining: Option<f32>,
    pub players_locked: Vec<String>,
    pub all_players: Vec<String>,
    pub guesses: Vec<GuessEntry>,
}

impl BettingScreen {
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            units: None,
            seconds_remaining: None,
            players_locked: Vec::new(),
            all_players: Vec::new(),
            guesses: Vec::new(),
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

    pub fn with_players_locked(mut self, players: Vec<String>) -> Self {
        self.players_locked = players;
        self
    }

    pub fn with_all_players(mut self, players: Vec<String>) -> Self {
        self.all_players = players;
        self
    }

    pub fn with_guesses(mut self, guesses: Vec<GuessEntry>) -> Self {
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

    fn locked_text(&self) -> String {
        if self.players_locked.is_empty() {
            "LOCKED:".to_string()
        } else {
            let mut locked = self.players_locked.clone();
            locked.sort();
            format!("LOCKED: {}", locked.join(", "))
        }
    }

    fn guess_sentences(&self, entry: &GuessEntry) -> Vec<String> {
        let mut sentences = Vec::new();

        // Line 1: Submitters (sorted, comma-separated) or LOWBALL
        if entry.value == 0 {
            sentences.push("LOWBALL".to_string());
        } else {
            let mut guessers = entry.guessers.clone();
            guessers.sort();
            sentences.push(guessers.join(", "));
        }

        // Line 2: The guess value
        sentences.push(entry.value.to_string());

        // Lines 3+: All players with their bet amount (sorted by name)
        let mut all_sorted = self.all_players.clone();
        all_sorted.sort();

        for player in &all_sorted {
            let amount = entry
                .bets
                .iter()
                .find(|b| &b.player == player)
                .map(|b| b.amount)
                .unwrap_or(0);
            sentences.push(format!("{}: {}", player, amount));
        }

        sentences
    }
}

#[derive(Component)]
struct BettingQuestionText;

#[derive(Component)]
struct BettingTimerText;

#[derive(Component)]
struct BettingLockedText;

#[derive(Component)]
struct BettingGuessList {
    index: usize,
}

fn calculate_guess_x(index: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let slot_width = 800.0 / total as f32;
    -400.0 + (index as f32 + 0.5) * slot_width
}

fn spawn_guess_lists(commands: &mut Commands, entity: Entity, screen: &BettingScreen) {
    let total = screen.guesses.len();
    commands.entity(entity).with_children(|parent| {
        for (index, entry) in screen.guesses.iter().enumerate() {
            let x = calculate_guess_x(index, total);
            parent.spawn((
                Name::new(format!("Guess_{}", index)),
                BettingGuessList { index },
                SentenceList::new(screen.guess_sentences(entry))
                    .with_text_speed(TEXT_SPEED)
                    .with_size(AnimatedTextSize::Small),
                Transform::from_xyz(x, -90.0, 0.0),
                Visibility::Inherited,
            ));
        }
    });
}

fn handle_betting_screen_added(
    trigger: On<Add, BettingScreen>,
    screen_q: Query<&BettingScreen>,
    mut commands: Commands,
) {
    let screen = screen_q
        .get(trigger.entity)
        .expect("BettingScreen should exist");

    commands.entity(trigger.entity).with_children(|parent| {
        // Question: 800x300 area, center at y=250
        parent.spawn((
            Name::new("Question"),
            BettingQuestionText,
            AnimatedText::new(&screen.question_text(), UVec2::new(768, 288), TEXT_SPEED),
            Transform::from_xyz(0.0, 250.0, 0.0),
            Visibility::Inherited,
        ));

        // Timer: 256x20 area, center at y=80
        parent.spawn((
            Name::new("Timer"),
            BettingTimerText,
            AnimatedText::new(&screen.timer_text(), UVec2::new(256, 20), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, 80.0, 0.0),
            Visibility::Inherited,
        ));

        // Locked: 768x20 area, center at y=55
        parent.spawn((
            Name::new("Locked"),
            BettingLockedText,
            AnimatedText::new(&screen.locked_text(), UVec2::new(768, 20), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, 55.0, 0.0),
            Visibility::Inherited,
        ));
    });

    // Spawn guess lists separately (they're dynamic)
    spawn_guess_lists(&mut commands, trigger.entity, screen);
}

fn update_betting_screen(
    mut commands: Commands,
    screen_q: Query<(Entity, &BettingScreen, &Children), Changed<BettingScreen>>,
    mut question_q: Query<&mut AnimatedText, With<BettingQuestionText>>,
    mut timer_q: Query<&mut AnimatedText, (With<BettingTimerText>, Without<BettingQuestionText>)>,
    mut locked_q: Query<
        &mut AnimatedText,
        (
            With<BettingLockedText>,
            Without<BettingQuestionText>,
            Without<BettingTimerText>,
        ),
    >,
    mut guess_q: Query<(Entity, &BettingGuessList, &mut SentenceList)>,
) {
    for (entity, screen, children) in &screen_q {
        // Update static text elements
        for child in children.iter() {
            if let Ok(mut text) = question_q.get_mut(child) {
                text.text_this_frame = screen.question_text();
            }
            if let Ok(mut text) = timer_q.get_mut(child) {
                text.text_this_frame = screen.timer_text();
            }
            if let Ok(mut text) = locked_q.get_mut(child) {
                text.text_this_frame = screen.locked_text();
            }
        }

        // Count current guess lists
        let current_guess_count = children.iter().filter(|c| guess_q.get(*c).is_ok()).count();
        let new_guess_count = screen.guesses.len();

        if current_guess_count != new_guess_count {
            // Despawn old guess lists and respawn
            for child in children.iter() {
                if guess_q.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
            spawn_guess_lists(&mut commands, entity, screen);
        } else {
            // Update existing guess lists by index
            for child in children.iter() {
                if let Ok((_, guess_list, mut sentence_list)) = guess_q.get_mut(child) {
                    if let Some(entry) = screen.guesses.get(guess_list.index) {
                        sentence_list.sentences = screen.guess_sentences(entry);
                    }
                }
            }
        }
    }
}

pub fn betting_plugin_fn(app: &mut App) {
    app.register_type::<BetEntry>();
    app.register_type::<GuessEntry>();
    app.add_observer(handle_betting_screen_added);
    app.add_systems(Update, update_betting_screen);
}
