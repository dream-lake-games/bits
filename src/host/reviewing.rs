//! ReviewingScreen - displays answer, winner, scores, and continue status.

use crate::prelude::*;
use bevy::prelude::*;

use super::betting::BetEntry;

const TEXT_SPEED: f32 = 0.02;

/// A player's score with their delta this round.
#[derive(Clone, Default, Reflect)]
pub struct ScoreEntry {
    pub player: String,
    pub score: i32,
    pub delta: i32,
}

/// Data for the reviewing phase screen.
#[derive(Component, Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
#[require(Transform, Visibility)]
pub struct ReviewingScreen {
    pub question: String,
    pub units: Option<String>,
    pub answer: u32,
    pub winning_guess: u32,
    pub winning_guessers: Vec<String>,
    pub winning_bets: Vec<BetEntry>,
    pub scores: Vec<ScoreEntry>,
    pub seconds_until_continue: Option<f32>,
    pub players_voted: Vec<String>,
}

impl ReviewingScreen {
    pub fn new(question: impl Into<String>, answer: u32) -> Self {
        Self {
            question: question.into(),
            units: None,
            answer,
            winning_guess: 0,
            winning_guessers: Vec::new(),
            winning_bets: Vec::new(),
            scores: Vec::new(),
            seconds_until_continue: None,
            players_voted: Vec::new(),
        }
    }

    pub fn with_units(mut self, units: impl Into<String>) -> Self {
        self.units = Some(units.into());
        self
    }

    pub fn with_winning_guess(mut self, guess: u32, guessers: Vec<String>) -> Self {
        self.winning_guess = guess;
        self.winning_guessers = guessers;
        self
    }

    pub fn with_winning_bets(mut self, bets: Vec<BetEntry>) -> Self {
        self.winning_bets = bets;
        self
    }

    pub fn with_scores(mut self, scores: Vec<ScoreEntry>) -> Self {
        self.scores = scores;
        self
    }

    pub fn with_seconds_until_continue(mut self, seconds: f32) -> Self {
        self.seconds_until_continue = Some(seconds);
        self
    }

    pub fn with_players_voted(mut self, players: Vec<String>) -> Self {
        self.players_voted = players;
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
        let seconds = self.seconds_until_continue.unwrap_or(0.0).max(0.0);
        format!("TIME {:.0}", seconds)
    }

    fn continue_text(&self) -> String {
        if self.players_voted.is_empty() {
            "CONTINUE:".to_string()
        } else {
            let mut voted = self.players_voted.clone();
            voted.sort();
            format!("CONTINUE: {}", voted.join(", "))
        }
    }

    fn correct_text(&self) -> String {
        if self.winning_guessers.is_empty() {
            format!("CORRECT: {}", self.answer)
        } else {
            let mut guessers = self.winning_guessers.clone();
            guessers.sort();
            format!("CORRECT: {} ({})", self.answer, guessers.join(", "))
        }
    }

    fn scores_sentences(&self) -> Vec<String> {
        let mut sentences = vec!["SCORES".to_string()];

        let mut sorted_scores = self.scores.clone();
        sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

        for (idx, entry) in sorted_scores.iter().enumerate() {
            let rank = idx + 1;
            let delta_str = if entry.delta >= 0 {
                format!("+{}", entry.delta)
            } else {
                format!("{}", entry.delta)
            };
            sentences.push(format!(
                "{}. {} - {} ({})",
                rank, entry.player, entry.score, delta_str
            ));
        }

        sentences
    }
}

#[derive(Component)]
struct ReviewingQuestionText;

#[derive(Component)]
struct ReviewingTimerText;

#[derive(Component)]
struct ReviewingContinueText;

#[derive(Component)]
struct ReviewingCorrectText;

#[derive(Component)]
struct ReviewingScoresList;

fn handle_reviewing_screen_added(
    trigger: On<Add, ReviewingScreen>,
    screen_q: Query<&ReviewingScreen>,
    mut commands: Commands,
) {
    let screen = screen_q
        .get(trigger.entity)
        .expect("ReviewingScreen should exist");

    commands.entity(trigger.entity).with_children(|parent| {
        // Question: 800x300 area, center at y=250
        parent.spawn((
            Name::new("Question"),
            ReviewingQuestionText,
            AnimatedText::new(&screen.question_text(), UVec2::new(768, 288), TEXT_SPEED),
            Transform::from_xyz(0.0, 250.0, 0.0),
            Visibility::Inherited,
        ));

        // Timer: 800x40 area, center at y=80
        parent.spawn((
            Name::new("Timer"),
            ReviewingTimerText,
            AnimatedText::new(&screen.timer_text(), UVec2::new(256, 32), TEXT_SPEED),
            Transform::from_xyz(0.0, 80.0, 0.0),
            Visibility::Inherited,
        ));

        // Continue: 800x40 area, center at y=40
        parent.spawn((
            Name::new("Continue"),
            ReviewingContinueText,
            AnimatedText::new(&screen.continue_text(), UVec2::new(768, 32), TEXT_SPEED),
            Transform::from_xyz(0.0, 40.0, 0.0),
            Visibility::Inherited,
        ));

        // Correct: 800x40 area, center at y=0
        parent.spawn((
            Name::new("Correct"),
            ReviewingCorrectText,
            AnimatedText::new(&screen.correct_text(), UVec2::new(768, 32), TEXT_SPEED),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Inherited,
        ));

        // Scores list: full 800px width, center at y=-130
        parent.spawn((
            Name::new("ScoresList"),
            ReviewingScoresList,
            SentenceList::new(screen.scores_sentences()).with_text_speed(TEXT_SPEED),
            Transform::from_xyz(0.0, -130.0, 0.0),
            Visibility::Inherited,
        ));
    });
}

fn update_reviewing_screen(
    screen_q: Query<(&ReviewingScreen, &Children), Changed<ReviewingScreen>>,
    mut question_q: Query<&mut AnimatedText, With<ReviewingQuestionText>>,
    mut timer_q: Query<
        &mut AnimatedText,
        (With<ReviewingTimerText>, Without<ReviewingQuestionText>),
    >,
    mut continue_q: Query<
        &mut AnimatedText,
        (
            With<ReviewingContinueText>,
            Without<ReviewingQuestionText>,
            Without<ReviewingTimerText>,
        ),
    >,
    mut correct_q: Query<
        &mut AnimatedText,
        (
            With<ReviewingCorrectText>,
            Without<ReviewingQuestionText>,
            Without<ReviewingTimerText>,
            Without<ReviewingContinueText>,
        ),
    >,
    mut scores_q: Query<&mut SentenceList, With<ReviewingScoresList>>,
) {
    for (screen, children) in &screen_q {
        for child in children.iter() {
            if let Ok(mut text) = question_q.get_mut(child) {
                text.text_this_frame = screen.question_text();
            }
            if let Ok(mut text) = timer_q.get_mut(child) {
                text.text_this_frame = screen.timer_text();
            }
            if let Ok(mut text) = continue_q.get_mut(child) {
                text.text_this_frame = screen.continue_text();
            }
            if let Ok(mut text) = correct_q.get_mut(child) {
                text.text_this_frame = screen.correct_text();
            }
            if let Ok(mut list) = scores_q.get_mut(child) {
                list.sentences = screen.scores_sentences();
            }
        }
    }
}

pub fn reviewing_plugin_fn(app: &mut App) {
    app.register_type::<ScoreEntry>();
    app.add_observer(handle_reviewing_screen_added);
    app.add_systems(Update, update_reviewing_screen);
}
