//! GuessingScreen - displays the question, timer, and player submission status.

use crate::prelude::*;
use bevy::prelude::*;

const TEXT_SPEED: f32 = 0.02;

/// Data for the guessing phase screen.
#[derive(Component, Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
#[require(Transform, Visibility)]
pub struct GuessingScreen {
    pub question: String,
    pub units: Option<String>,
    pub seconds_remaining: Option<f32>,
    pub players_waiting: Vec<String>,
    pub players_submitted: Vec<String>,
}

impl GuessingScreen {
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            units: None,
            seconds_remaining: None,
            players_waiting: Vec::new(),
            players_submitted: Vec::new(),
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

    pub fn with_players_waiting(mut self, players: Vec<String>) -> Self {
        self.players_waiting = players;
        self
    }

    pub fn with_players_submitted(mut self, players: Vec<String>) -> Self {
        self.players_submitted = players;
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

    fn waiting_sentences(&self) -> Vec<String> {
        let mut sentences = vec!["WAITING:".to_string()];
        let mut waiting = self.players_waiting.clone();
        waiting.sort();
        sentences.extend(waiting.into_iter().map(|p| format!("*{}", p)));
        sentences
    }

    fn submitted_sentences(&self) -> Vec<String> {
        let mut sentences = vec!["SUBMITTED:".to_string()];
        let mut submitted = self.players_submitted.clone();
        submitted.sort();
        sentences.extend(submitted.into_iter().map(|p| format!("*{}", p)));
        sentences
    }
}

#[derive(Component)]
struct GuessingQuestionText;

#[derive(Component)]
struct GuessingTimerText;

#[derive(Component)]
struct GuessingWaitingList;

#[derive(Component)]
struct GuessingSubmittedList;

fn handle_guessing_screen_added(
    trigger: On<Add, GuessingScreen>,
    screen_q: Query<&GuessingScreen>,
    mut commands: Commands,
) {
    let screen = screen_q
        .get(trigger.entity)
        .expect("GuessingScreen should exist");

    commands.entity(trigger.entity).with_children(|parent| {
        // Question: 800x300 area, center at y=250
        parent.spawn((
            Name::new("Question"),
            GuessingQuestionText,
            AnimatedText::new(&screen.question_text(), UVec2::new(768, 288), TEXT_SPEED),
            Transform::from_xyz(0.0, 250.0, 0.0),
            Visibility::Inherited,
        ));

        // Timer: 256x20 area, center at y=80
        parent.spawn((
            Name::new("Timer"),
            GuessingTimerText,
            AnimatedText::new(&screen.timer_text(), UVec2::new(256, 20), TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(0.0, 80.0, 0.0),
            Visibility::Inherited,
        ));

        // Waiting list: 400x300 area, center at x=-200, y=-70
        parent.spawn((
            Name::new("WaitingList"),
            GuessingWaitingList,
            SentenceList::new(screen.waiting_sentences())
                .with_text_speed(TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(-200.0, -70.0, 0.0),
            Visibility::Inherited,
        ));

        // Submitted list: 400x300 area, center at x=200, y=-70
        parent.spawn((
            Name::new("SubmittedList"),
            GuessingSubmittedList,
            SentenceList::new(screen.submitted_sentences())
                .with_text_speed(TEXT_SPEED)
                .with_size(AnimatedTextSize::Small),
            Transform::from_xyz(200.0, -70.0, 0.0),
            Visibility::Inherited,
        ));
    });
}

fn update_guessing_screen(
    screen_q: Query<(&GuessingScreen, &Children), Changed<GuessingScreen>>,
    mut question_q: Query<&mut AnimatedText, With<GuessingQuestionText>>,
    mut timer_q: Query<&mut AnimatedText, (With<GuessingTimerText>, Without<GuessingQuestionText>)>,
    mut waiting_q: Query<&mut SentenceList, With<GuessingWaitingList>>,
    mut submitted_q: Query<
        &mut SentenceList,
        (With<GuessingSubmittedList>, Without<GuessingWaitingList>),
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
            if let Ok(mut list) = waiting_q.get_mut(child) {
                list.sentences = screen.waiting_sentences();
            }
            if let Ok(mut list) = submitted_q.get_mut(child) {
                list.sentences = screen.submitted_sentences();
            }
        }
    }
}

pub fn guessing_plugin_fn(app: &mut App) {
    app.add_observer(handle_guessing_screen_added);
    app.add_systems(Update, update_guessing_screen);
}
