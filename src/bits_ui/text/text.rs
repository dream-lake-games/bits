use std::collections::HashMap;

use crate::bits_ui::anim::AnimMan;
use bevy::{ecs::system::BoxedSystem, prelude::*};

use super::letters::{LETTER_SIZE, char_to_letter_anim};

fn split_text_into_lines(text: String, max_chars_per_line: u32, max_lines: u32) -> Vec<String> {
    let words = text.split_ascii_whitespace();
    let mut result = vec![];

    let mut current_line = String::new();

    for word in words {
        if word.len() > max_chars_per_line as usize {
            if !current_line.is_empty() {
                result.push(current_line.clone());
            }
            warn!("Encountered word longer than line while splitting text");
            result.push(word.to_string());
            current_line.clear();
        } else if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() > max_chars_per_line as usize {
            result.push(current_line.clone());
            current_line = word.to_string();
        } else {
            current_line.push(' ');
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.len() > max_lines as usize {
        warn!("Encountered text that overflowed vertically");
    }

    result
}

fn get_char_offsets(lines: Vec<String>) -> HashMap<IVec2, char> {
    let mut result = HashMap::new();
    let num_lines = lines.len();

    for (line_idx, line) in lines.iter().enumerate() {
        let num_chars = line.chars().count();
        let y = (((num_lines - 1) as f32 / 2.0) - line_idx as f32) * LETTER_SIZE as f32;

        for (char_idx, ch) in line.chars().enumerate() {
            let x = (char_idx as f32 - (num_chars - 1) as f32 / 2.0) * LETTER_SIZE as f32;
            result.insert(IVec2::new(x as i32, y as i32), ch);
        }
    }

    result
}

#[derive(Component)]
struct LetterMarker {
    letter: char,
    offset: IVec2,
}

#[derive(Debug, Clone)]
enum LetterState {
    Pending(char, f32),
    Spawned(char),
}

#[derive(Component, Debug)]
pub struct AnimatedText {
    pub text_system: Option<BoxedSystem<(), String>>,
    seconds_per_char: f32,
    pub text_last_frame: Option<String>,
    pub text_this_frame: String,
    size: UVec2,
    char_state: HashMap<IVec2, LetterState>,
}

impl AnimatedText {
    pub fn new(text: impl Into<String>, size: UVec2, seconds_per_char: f32) -> Self {
        Self {
            text_system: None,
            seconds_per_char,
            text_last_frame: None,
            text_this_frame: text.into(),
            size,
            char_state: HashMap::new(),
        }
    }

    pub fn with_text_system<M>(mut self, system: impl IntoSystem<(), String, M> + 'static) -> Self {
        self.text_system = Some(Box::new(IntoSystem::into_system(system)));
        self
    }

    fn max_chars_per_line(&self) -> u32 {
        self.size.x / LETTER_SIZE
    }

    fn max_lines(&self) -> u32 {
        self.size.y / LETTER_SIZE
    }
}

fn update_text_this_frame(world: &mut World) {
    let mut texts_to_check = Vec::new();
    {
        let mut query = world.query::<(Entity, &AnimatedText)>();
        for (entity, text) in query.iter(world) {
            if text.text_system.is_some() {
                texts_to_check.push(entity);
            }
        }
    }

    for entity in texts_to_check {
        let system_opt = world
            .get_mut::<AnimatedText>(entity)
            .expect("AnimatedText entity should exist")
            .text_system
            .take();

        if let Some(mut text_system) = system_opt {
            text_system.initialize(world);
            let new_text = text_system
                .run((), world)
                .expect("AnimatedText text_system should run successfully");
            text_system.apply_deferred(world);

            let mut text = world
                .get_mut::<AnimatedText>(entity)
                .expect("AnimatedText entity should exist");

            text.text_this_frame = new_text;
            text.text_system = Some(text_system);
        }
    }
}

fn maybe_reset_char_state(mut query: Query<&mut AnimatedText>) {
    for mut text in query.iter_mut() {
        let text_changed = text.text_last_frame.as_ref() != Some(&text.text_this_frame);
        if !text_changed {
            continue;
        }

        let lines = split_text_into_lines(
            text.text_this_frame.clone(),
            text.max_chars_per_line(),
            text.max_lines(),
        );
        let char_offsets = get_char_offsets(lines);

        let mut sorted_offsets: Vec<_> = char_offsets.into_iter().collect();
        sorted_offsets.sort_by(|(a, _), (b, _)| match b.y.cmp(&a.y) {
            std::cmp::Ordering::Equal => a.x.cmp(&b.x),
            other => other,
        });

        let mut new_char_state = HashMap::new();
        let mut pending_count = 0u32;

        for (offset, ch) in sorted_offsets.into_iter() {
            let already_spawned = matches!(
                text.char_state.get(&offset),
                Some(LetterState::Spawned(existing_ch)) if *existing_ch == ch
            );

            if already_spawned {
                new_char_state.insert(offset, LetterState::Spawned(ch));
            } else {
                let delay = pending_count as f32 * text.seconds_per_char;
                new_char_state.insert(offset, LetterState::Pending(ch, delay));
                pending_count += 1;
            }
        }

        text.char_state = new_char_state;
        text.text_last_frame = Some(text.text_this_frame.clone());
    }
}

fn tick_char_state(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimatedText)>,
) {
    for (entity, mut text) in query.iter_mut() {
        let mut to_spawn = Vec::new();

        for (offset, state) in text.char_state.iter_mut() {
            if let LetterState::Pending(ch, time_left) = state {
                *time_left -= time.delta_secs();
                if *time_left <= 0.0 {
                    to_spawn.push((*offset, *ch));
                }
            }
        }

        for (offset, ch) in to_spawn {
            text.char_state.insert(offset, LetterState::Spawned(ch));

            let letter_anim = char_to_letter_anim(ch);
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    LetterMarker { letter: ch, offset },
                    AnimMan::new(letter_anim),
                    Transform::from_translation(offset.extend(0).as_vec3()),
                    Visibility::Inherited,
                ));
            });
        }
    }
}

fn update_letter_children(
    mut commands: Commands,
    letter_query: Query<(Entity, &LetterMarker, &ChildOf)>,
    text_query: Query<&AnimatedText>,
) {
    for (letter_entity, marker, child_of) in letter_query.iter() {
        let text = text_query
            .get(child_of.parent())
            .expect("Letter's parent must have AnimatedText component");

        let is_blessed = match text.char_state.get(&marker.offset) {
            Some(LetterState::Spawned(ch)) => *ch == marker.letter,
            _ => false,
        };

        if !is_blessed {
            commands.entity(letter_entity).despawn();
        }
    }
}

pub fn text_plugin_fn(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_text_this_frame,
            maybe_reset_char_state,
            tick_char_state,
            update_letter_children,
        )
            .chain(),
    );
}
