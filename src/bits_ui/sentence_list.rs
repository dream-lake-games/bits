//! SentenceList - vertically stacked sentences using AnimatedText.

use bevy::prelude::*;

use super::text::{AnimatedText, AnimatedTextSize};

const DEFAULT_TEXT_SPEED: f32 = 0.02;
const BASE_ROW_HEIGHT: f32 = 40.0;

/// A vertically stacked list of sentences, centered around its transform origin.
#[derive(Component, Reflect, Default, Clone)]
#[require(Transform, Visibility)]
pub struct SentenceList {
    pub sentences: Vec<String>,
    text_speed: f32,
    text_size: AnimatedTextSize,
}

impl SentenceList {
    pub fn new(sentences: Vec<String>) -> Self {
        Self {
            sentences,
            text_speed: DEFAULT_TEXT_SPEED,
            text_size: AnimatedTextSize::default(),
        }
    }

    pub fn with_text_speed(mut self, speed: f32) -> Self {
        self.text_speed = speed;
        self
    }

    pub fn with_size(mut self, text_size: AnimatedTextSize) -> Self {
        self.text_size = text_size;
        self
    }

    fn text_speed(&self) -> f32 {
        if self.text_speed <= 0.0 {
            DEFAULT_TEXT_SPEED
        } else {
            self.text_speed
        }
    }

    fn row_height(&self) -> f32 {
        BASE_ROW_HEIGHT * self.text_size.scale()
    }
}

#[derive(Component)]
struct SentenceRow {
    index: usize,
}

fn calculate_row_y(index: usize, total_count: usize, row_height: f32) -> f32 {
    if total_count == 0 {
        return 0.0;
    }
    let total_height = (total_count - 1) as f32 * row_height;
    let top_y = total_height / 2.0;
    top_y - (index as f32 * row_height)
}

fn calculate_text_width(sentence: &str, letter_size: f32) -> u32 {
    let char_count = sentence.chars().count() as u32;
    (char_count.max(1) as f32 * letter_size) as u32
}

fn spawn_sentence_children(commands: &mut Commands, entity: Entity, list: &SentenceList) {
    let letter_size = list.text_size.letter_size();
    let row_height = list.row_height();

    commands.entity(entity).with_children(|parent| {
        let total = list.sentences.len();
        for (index, sentence) in list.sentences.iter().enumerate() {
            let width = calculate_text_width(sentence, letter_size);
            let y = calculate_row_y(index, total, row_height);

            parent.spawn((
                Name::new(format!("Sentence_{}", index)),
                SentenceRow { index },
                AnimatedText::new(
                    sentence,
                    UVec2::new(width, letter_size as u32),
                    list.text_speed(),
                )
                .with_size(list.text_size),
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
