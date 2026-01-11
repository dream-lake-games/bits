//! SentenceList - vertically stacked sentences using AnimatedText.

use bevy::prelude::*;

use super::LETTER_SIZE;
use super::text::AnimatedText;

const DEFAULT_TEXT_SPEED: f32 = 0.02;
const ROW_HEIGHT: f32 = 40.0;

/// A vertically stacked list of sentences, centered around its transform origin.
#[derive(Component, Reflect, Default, Clone)]
#[require(Transform, Visibility)]
pub struct SentenceList {
    pub sentences: Vec<String>,
    text_speed: f32,
}

impl SentenceList {
    pub fn new(sentences: Vec<String>) -> Self {
        Self {
            sentences,
            text_speed: DEFAULT_TEXT_SPEED,
        }
    }

    pub fn with_text_speed(mut self, speed: f32) -> Self {
        self.text_speed = speed;
        self
    }

    fn text_speed(&self) -> f32 {
        if self.text_speed <= 0.0 {
            DEFAULT_TEXT_SPEED
        } else {
            self.text_speed
        }
    }
}

#[derive(Component)]
struct SentenceRow {
    index: usize,
}

fn calculate_row_y(index: usize, total_count: usize) -> f32 {
    if total_count == 0 {
        return 0.0;
    }
    let total_height = (total_count - 1) as f32 * ROW_HEIGHT;
    let top_y = total_height / 2.0;
    top_y - (index as f32 * ROW_HEIGHT)
}

fn calculate_text_width(sentence: &str) -> u32 {
    let char_count = sentence.chars().count() as u32;
    char_count.max(1) * LETTER_SIZE
}

fn spawn_sentence_children(commands: &mut Commands, entity: Entity, list: &SentenceList) {
    commands.entity(entity).with_children(|parent| {
        let total = list.sentences.len();
        for (index, sentence) in list.sentences.iter().enumerate() {
            let width = calculate_text_width(sentence);
            let y = calculate_row_y(index, total);

            parent.spawn((
                Name::new(format!("Sentence_{}", index)),
                SentenceRow { index },
                AnimatedText::new(sentence, UVec2::new(width, LETTER_SIZE), list.text_speed()),
                Transform::from_xyz(0.0, y, 0.0),
                Visibility::Inherited,
            ));
        }
    });
}

fn handle_sentence_list_added(
    trigger: On<Add, SentenceList>,
    list_q: Query<&SentenceList>,
    mut commands: Commands,
) {
    let list = list_q
        .get(trigger.entity)
        .expect("SentenceList should exist");

    spawn_sentence_children(&mut commands, trigger.entity, list);
}

fn update_sentence_list(
    mut commands: Commands,
    list_q: Query<(Entity, &SentenceList, &Children), Changed<SentenceList>>,
    mut row_q: Query<(&SentenceRow, &mut AnimatedText)>,
) {
    for (entity, list, children) in &list_q {
        let current_count = children.len();
        let new_count = list.sentences.len();

        if current_count != new_count {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
            spawn_sentence_children(&mut commands, entity, list);
            continue;
        }

        for child in children.iter() {
            let Ok((row, mut text)) = row_q.get_mut(child) else {
                continue;
            };
            if let Some(sentence) = list.sentences.get(row.index) {
                text.text_this_frame = sentence.clone();
            }
        }
    }
}

pub fn sentence_list_plugin_fn(app: &mut App) {
    app.register_type::<SentenceList>();
    app.add_observer(handle_sentence_list_added);
    app.add_systems(Update, update_sentence_list);
}
