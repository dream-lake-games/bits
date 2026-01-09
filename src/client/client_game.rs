use std::time::Duration;

use bevy::prelude::*;
use bits::prelude::*;

use crate::{
    client_simple::InputsQueue,
    client_state::{ClientConnectionState, ClientGameState},
};

#[derive(Component)]
struct WaitingForQuestionCleanup;

fn waiting_for_question_text(
    mut timer: Local<Option<Timer>>,
    time: Res<Time>,
    mut dot_thing: Local<u32>,
) -> String {
    let timer_duration = Duration::from_secs_f32(0.33);
    if timer.is_none() {
        *timer = Some(Timer::new(timer_duration.clone(), TimerMode::Once));
    }
    let timer = timer.as_mut().unwrap();
    timer.tick(time.delta());
    if timer.is_finished() {
        *dot_thing = (*dot_thing + 1) % 3;
        *timer = Timer::new(timer_duration.clone(), TimerMode::Once);
    }
    format!(
        "Waiting for question{}",
        ".".repeat((*dot_thing + 1) as usize)
    )
}

fn on_enter_waiting_for_question(mut commands: Commands) {
    commands.spawn((
        FlexSimple::new().bundle(),
        WaitingForQuestionCleanup,
        children![
            TextSimple::p("")
                .with_text_system(waiting_for_question_text)
                .bundle(),
        ],
    ));
}

fn on_exit_waiting_for_question(
    cleanup_q: Query<Entity, With<WaitingForQuestionCleanup>>,
    mut commands: Commands,
) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

#[derive(Component)]
struct GuessingCleanup;

#[derive(Component, Debug, Clone, Reflect, Default)]
struct PendingGuess {
    current_number: String,
    submit_sent: bool,
}

fn guess_time_remaining_text(question_q: Query<&QuestionActive>) -> String {
    if let Ok(active) = question_q.single() {
        let seconds = active
            .guess_seconds_remaining
            .map(|s| s.max(0.0))
            .unwrap_or(0.0);
        format!("Time: {:.1}s", seconds)
    } else {
        "Time: 0.0s".to_string()
    }
}

fn is_input_disabled(pending_guess: Single<&PendingGuess>) -> bool {
    pending_guess.submit_sent
}

fn is_submit_disabled(pending_guess: Single<&PendingGuess>) -> bool {
    if pending_guess.submit_sent {
        return true;
    }
    if pending_guess.current_number.is_empty() {
        return true;
    }
    if let Ok(value) = pending_guess.current_number.parse::<u32>() {
        if value == 0 {
            return true;
        }
    }
    false
}

fn handle_submit_guess(
    mut pending_guess: Single<&mut PendingGuess>,
    mut inputs_queue: ResMut<InputsQueue>,
) {
    if let Ok(guess) = pending_guess.current_number.parse::<u32>() {
        inputs_queue
            .queue
            .push_back(ClientInput::SubmitGuess { guess });
        pending_guess.submit_sent = true;
    }
}

fn on_enter_guessing(mut commands: Commands) {
    commands.spawn((
        Name::new("PendingGuess"),
        PendingGuess::default(),
        GuessingCleanup,
    ));

    const SPACING: f32 = 10.0;
    const SECTION_SPACING: f32 = 20.0;
    const BUTTON_WIDTH: f32 = 80.0;
    const BUTTON_HEIGHT: f32 = 50.0;
    const GUESS_WIDTH: f32 = 200.0;
    const GUESS_HEIGHT: f32 = 50.0;
    const SUBMIT_WIDTH: f32 = 270.0;
    const SUBMIT_HEIGHT: f32 = 50.0;
    const CLEAR_FONT_SIZE: f32 = 18.0;

    let spacing = Val::Px(SPACING);
    let section_spacing = Val::Px(SECTION_SPACING);

    let number_button = |num: &str| {
        let num_string = num.to_string();
        ButtonSimple::small(num)
            .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
            .with_disabled_system(is_input_disabled)
            .with_on_release(move |mut pending_guess: Single<&mut PendingGuess>| {
                let new_value = format!("{}{}", pending_guess.current_number, num_string);
                if let Ok(parsed) = new_value.parse::<u32>() {
                    let as_string = parsed.to_string();
                    if as_string.len() <= 6 {
                        pending_guess.current_number = as_string;
                    }
                }
            })
            .bundle()
    };

    commands.spawn((
        FlexSimple::new()
            .with_visibility_system(
                |connection_state: Res<State<ClientConnectionState>>,
                 question_q: Query<&Question, With<QuestionActive>>| {
                    if let ClientConnectionState::Named { username } = connection_state.get() {
                        if let Ok(question) = question_q.single() {
                            return !question.guesses.contains_key(username);
                        }
                    }
                    true
                },
            )
            .bundle(),
        GuessingCleanup,
        children![
            TextSimple::p("")
                .with_text_system(guess_time_remaining_text)
                .bundle(),
            Spacer::height(section_spacing).bundle(),
            TextSimple::p("")
                .with_size(Val::Px(GUESS_WIDTH), Val::Px(GUESS_HEIGHT))
                .with_bg_color(Color::srgb(0.1, 0.1, 0.15))
                .with_border(Val::Px(2.0), Color::srgb(0.3, 0.3, 0.4))
                .with_text_system(|pending_guess: Single<&PendingGuess>| {
                    if pending_guess.current_number.is_empty() {
                        "0".to_string()
                    } else {
                        pending_guess.current_number.clone()
                    }
                })
                .bundle(),
            Spacer::height(section_spacing).bundle(),
            (
                FlexSimple::new()
                    .with_direction(FlexDirection::Column)
                    .with_size(Val::Auto, Val::Auto)
                    .bundle(),
                children![
                    // Row: 1 2 3
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            number_button("1"),
                            Spacer::width(spacing).bundle(),
                            number_button("2"),
                            Spacer::width(spacing).bundle(),
                            number_button("3"),
                        ],
                    ),
                    Spacer::height(spacing).bundle(),
                    // Row: 4 5 6
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            number_button("4"),
                            Spacer::width(spacing).bundle(),
                            number_button("5"),
                            Spacer::width(spacing).bundle(),
                            number_button("6"),
                        ],
                    ),
                    Spacer::height(spacing).bundle(),
                    // Row: 7 8 9
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            number_button("7"),
                            Spacer::width(spacing).bundle(),
                            number_button("8"),
                            Spacer::width(spacing).bundle(),
                            number_button("9"),
                        ],
                    ),
                    Spacer::height(spacing).bundle(),
                    // Row: CLR 0 DEL
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            ButtonSimple::small("CLR")
                                .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
                                .with_font_size(CLEAR_FONT_SIZE)
                                .with_disabled_system(is_input_disabled)
                                .with_on_release(|mut pending_guess: Single<&mut PendingGuess>| {
                                    pending_guess.current_number.clear();
                                })
                                .bundle(),
                            Spacer::width(spacing).bundle(),
                            number_button("0"),
                            Spacer::width(spacing).bundle(),
                            ButtonSimple::small("DEL")
                                .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
                                .with_disabled_system(is_input_disabled)
                                .with_on_release(|mut pending_guess: Single<&mut PendingGuess>| {
                                    pending_guess.current_number.pop();
                                })
                                .bundle(),
                        ],
                    ),
                ],
            ),
            Spacer::height(section_spacing).bundle(),
            ButtonSimple::small("Submit")
                .with_size(Val::Px(SUBMIT_WIDTH), Val::Px(SUBMIT_HEIGHT))
                .with_disabled_system(is_submit_disabled)
                .with_on_release(handle_submit_guess)
                .bundle(),
        ],
    ));

    commands.spawn((
        FlexSimple::new()
            .with_visibility_system(
                |connection_state: Res<State<ClientConnectionState>>,
                 question_q: Query<&Question, With<QuestionActive>>| {
                    if let ClientConnectionState::Named { username } = connection_state.get() {
                        if let Ok(question) = question_q.single() {
                            return question.guesses.contains_key(username);
                        }
                    }
                    false
                },
            )
            .bundle(),
        GuessingCleanup,
        children![
            TextSimple::p("")
                .with_text_system(guess_time_remaining_text)
                .bundle(),
            TextSimple::p("")
                .with_text_system(
                    |connection_state: Res<State<ClientConnectionState>>,
                     question_q: Query<&Question, With<QuestionActive>>| {
                        if let ClientConnectionState::Named { username } = connection_state.get() {
                            if let Ok(question) = question_q.single() {
                                if let Some(&guess) = question.guesses.get(username) {
                                    return format!("Submitted: {}", guess);
                                }
                            }
                        }
                        "Submitted".to_string()
                    },
                )
                .bundle()
        ],
    ));

    commands.spawn((
        FlexSimple::new()
            .with_visibility_system(
                |connection_state: Res<State<ClientConnectionState>>,
                 question_q: Query<(&Question, &QuestionActive)>| {
                    if let ClientConnectionState::Named { username } = connection_state.get() {
                        if let Ok((question, active)) = question_q.single() {
                            return active.guess_seconds_remaining.is_none()
                                && !question.guesses.contains_key(username);
                        }
                    }
                    false
                },
            )
            .bundle(),
        GuessingCleanup,
        children![
            TextSimple::p("")
                .with_text_system(guess_time_remaining_text)
                .bundle(),
            TextSimple::p("No guess submitted").bundle()
        ],
    ));
}

fn update_guessing() {}

fn on_exit_guessing(cleanup_q: Query<Entity, With<GuessingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

#[derive(Component)]
struct BettingCleanup;

fn format_question_with_units(question: &Question) -> String {
    match &question.units {
        Some(units) => format!("{} (in {})", question.question, units),
        None => question.question.clone(),
    }
}

fn betting_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(format_question_with_units)
        .unwrap_or("(no question)".to_string())
}

fn betting_time_remaining_text(bets_active_q: Query<&BetsActive>) -> String {
    if let Ok(active) = bets_active_q.single() {
        let seconds = active
            .bets_seconds_remaining
            .map(|s| s.max(0.0))
            .unwrap_or(0.0);
        format!("Time: {:.1}s", seconds)
    } else {
        "Time: 0.0s".to_string()
    }
}

fn get_bet_for_user(bets: &Bets, guess_value: u32, username: &str) -> Option<Bet> {
    bets.bets
        .get(&guess_value)
        .and_then(|bet_list| bet_list.iter().find(|b| b.owner == username).cloned())
}

fn get_total_free_bets(bets: &Bets, username: &str) -> u32 {
    bets.bets
        .values()
        .flat_map(|bet_list| bet_list.iter())
        .filter(|b| b.owner == username)
        .map(|b| b.num_free)
        .sum()
}

fn get_total_paid_bets(bets: &Bets, username: &str) -> u32 {
    bets.bets
        .values()
        .flat_map(|bet_list| bet_list.iter())
        .filter(|b| b.owner == username)
        .map(|b| b.num_paid)
        .sum()
}

fn is_user_locked(bets_active: &BetsActive, username: &str) -> bool {
    bets_active
        .bets_locked
        .get(username)
        .copied()
        .unwrap_or(false)
}

fn on_enter_betting(
    mut commands: Commands,
    question_q: Query<&Question, With<QuestionActive>>,
    connection_state: Res<State<ClientConnectionState>>,
) {
    let ClientConnectionState::Named { username } = connection_state.get() else {
        warn!("Client not named when entering betting");
        return;
    };
    let current_username = username.clone();

    let Ok(question) = question_q.single() else {
        warn!("No active question when entering betting");
        return;
    };

    let mut guess_values: Vec<u32> = question.guesses.values().cloned().collect();
    guess_values.sort();
    guess_values.dedup();
    if !guess_values.contains(&0) {
        guess_values.insert(0, 0);
    }

    let num_boxes = guess_values.len();
    let box_width_percent = 100.0 / num_boxes as f32;

    const CONTROL_BUTTON_SIZE: f32 = 30.0;
    const CONTROL_SPACING: f32 = 5.0;
    const SECTION_SPACING: f32 = 10.0;
    const LABEL_FONT_SIZE: f32 = 12.0;
    const AMOUNT_FONT_SIZE: f32 = 14.0;
    const LOCK_BUTTON_WIDTH: f32 = 200.0;
    const LOCK_BUTTON_HEIGHT: f32 = 40.0;

    let row_container = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(60.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .id();

    for guess_value in guess_values {
        let username_for_free_text = current_username.clone();
        let username_for_paid_text = current_username.clone();
        let username_for_free_minus = current_username.clone();
        let username_for_free_plus = current_username.clone();
        let username_for_paid_minus = current_username.clone();
        let username_for_paid_plus = current_username.clone();
        let username_for_free_minus_release = current_username.clone();
        let username_for_free_plus_release = current_username.clone();
        let username_for_paid_minus_release = current_username.clone();
        let username_for_paid_plus_release = current_username.clone();

        let header_text = if guess_value == 0 {
            "LOWBALL".to_string()
        } else {
            let owners: Vec<&str> = question
                .guesses
                .iter()
                .filter(|(_, v)| **v == guess_value)
                .map(|(name, _)| name.as_str())
                .collect();
            format!("Owner(s): {}", owners.join(", "))
        };

        let box_entity = commands
            .spawn((
                Node {
                    width: Val::Percent(box_width_percent),
                    height: Val::Auto,
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
                BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
                children![
                    TextSimple::p(&header_text)
                        .with_font_size(LABEL_FONT_SIZE)
                        .bundle(),
                    TextSimple::p(format!("Guess: {}", guess_value))
                        .with_font_size(LABEL_FONT_SIZE)
                        .bundle(),
                    Spacer::height(Val::Px(SECTION_SPACING)).bundle(),
                    TextSimple::p("Free")
                        .with_font_size(LABEL_FONT_SIZE)
                        .bundle(),
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(CONTROL_SPACING),
                            ..default()
                        },
                        children![
                            ButtonSimple::small("-")
                                .with_size(
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                )
                                .with_font_size(LABEL_FONT_SIZE)
                                .with_disabled_system(
                                    move |bets_q: Query<(&Bets, &BetsActive)>| -> bool {
                                        let Ok((bets, bets_active)) = bets_q.single() else {
                                            return true;
                                        };
                                        if is_user_locked(bets_active, &username_for_free_minus) {
                                            return true;
                                        }
                                        let current_free = get_bet_for_user(
                                            bets,
                                            guess_value,
                                            &username_for_free_minus,
                                        )
                                        .map(|b| b.num_free)
                                        .unwrap_or(0);
                                        current_free == 0
                                    },
                                )
                                .with_on_release(
                                    move |mut inputs_queue: ResMut<InputsQueue>,
                                          bets_q: Query<&Bets, With<BetsActive>>| {
                                        let (current_free, current_paid) = bets_q
                                            .single()
                                            .ok()
                                            .and_then(|bets| {
                                                get_bet_for_user(
                                                    bets,
                                                    guess_value,
                                                    &username_for_free_minus_release,
                                                )
                                            })
                                            .map(|b| (b.num_free, b.num_paid))
                                            .unwrap_or((0, 0));
                                        inputs_queue.push(ClientInput::SubmitBet {
                                            guess: guess_value,
                                            num_free: current_free.saturating_sub(1),
                                            num_paid: current_paid,
                                        });
                                    },
                                )
                                .bundle(),
                            TextSimple::p("")
                                .with_font_size(AMOUNT_FONT_SIZE)
                                .with_text_system(
                                    move |bets_q: Query<&Bets, With<BetsActive>>| -> String {
                                        if let Ok(bets) = bets_q.single() {
                                            if let Some(bet) = get_bet_for_user(
                                                bets,
                                                guess_value,
                                                &username_for_free_text,
                                            ) {
                                                return bet.num_free.to_string();
                                            }
                                        }
                                        "0".to_string()
                                    },
                                )
                                .bundle(),
                            ButtonSimple::small("+")
                                .with_size(
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                )
                                .with_font_size(LABEL_FONT_SIZE)
                                .with_disabled_system(
                                    move |bets_q: Query<(&Bets, &BetsActive)>| -> bool {
                                        let Ok((bets, bets_active)) = bets_q.single() else {
                                            return true;
                                        };
                                        if is_user_locked(bets_active, &username_for_free_plus) {
                                            return true;
                                        }
                                        get_total_free_bets(bets, &username_for_free_plus) >= 2
                                    },
                                )
                                .with_on_release(
                                    move |mut inputs_queue: ResMut<InputsQueue>,
                                          bets_q: Query<&Bets, With<BetsActive>>| {
                                        let (current_free, current_paid) = bets_q
                                            .single()
                                            .ok()
                                            .and_then(|bets| {
                                                get_bet_for_user(
                                                    bets,
                                                    guess_value,
                                                    &username_for_free_plus_release,
                                                )
                                            })
                                            .map(|b| (b.num_free, b.num_paid))
                                            .unwrap_or((0, 0));
                                        inputs_queue.push(ClientInput::SubmitBet {
                                            guess: guess_value,
                                            num_free: current_free + 1,
                                            num_paid: current_paid,
                                        });
                                    },
                                )
                                .bundle(),
                        ],
                    ),
                    Spacer::height(Val::Px(SECTION_SPACING - 2.0)).bundle(),
                    TextSimple::p("Paid")
                        .with_font_size(LABEL_FONT_SIZE)
                        .bundle(),
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(CONTROL_SPACING),
                            ..default()
                        },
                        children![
                            ButtonSimple::small("-")
                                .with_size(
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                )
                                .with_font_size(LABEL_FONT_SIZE)
                                .with_disabled_system(
                                    move |bets_q: Query<(&Bets, &BetsActive)>| -> bool {
                                        let Ok((bets, bets_active)) = bets_q.single() else {
                                            return true;
                                        };
                                        if is_user_locked(bets_active, &username_for_paid_minus) {
                                            return true;
                                        }
                                        let current_paid = get_bet_for_user(
                                            bets,
                                            guess_value,
                                            &username_for_paid_minus,
                                        )
                                        .map(|b| b.num_paid)
                                        .unwrap_or(0);
                                        current_paid == 0
                                    },
                                )
                                .with_on_release(
                                    move |mut inputs_queue: ResMut<InputsQueue>,
                                          bets_q: Query<&Bets, With<BetsActive>>| {
                                        let (current_free, current_paid) = bets_q
                                            .single()
                                            .ok()
                                            .and_then(|bets| {
                                                get_bet_for_user(
                                                    bets,
                                                    guess_value,
                                                    &username_for_paid_minus_release,
                                                )
                                            })
                                            .map(|b| (b.num_free, b.num_paid))
                                            .unwrap_or((0, 0));
                                        inputs_queue.push(ClientInput::SubmitBet {
                                            guess: guess_value,
                                            num_free: current_free,
                                            num_paid: current_paid.saturating_sub(1),
                                        });
                                    },
                                )
                                .bundle(),
                            TextSimple::p("")
                                .with_font_size(AMOUNT_FONT_SIZE)
                                .with_text_system(
                                    move |bets_q: Query<&Bets, With<BetsActive>>| -> String {
                                        if let Ok(bets) = bets_q.single() {
                                            if let Some(bet) = get_bet_for_user(
                                                bets,
                                                guess_value,
                                                &username_for_paid_text,
                                            ) {
                                                return bet.num_paid.to_string();
                                            }
                                        }
                                        "0".to_string()
                                    },
                                )
                                .bundle(),
                            ButtonSimple::small("+")
                                .with_size(
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                    Val::Px(CONTROL_BUTTON_SIZE),
                                )
                                .with_font_size(LABEL_FONT_SIZE)
                                .with_disabled_system(
                                    move |bets_q: Query<(&Bets, &BetsActive)>,
                                          game_state_q: Query<&GameState>|
                                          -> bool {
                                        let Ok((bets, bets_active)) = bets_q.single() else {
                                            return true;
                                        };
                                        if is_user_locked(bets_active, &username_for_paid_plus) {
                                            return true;
                                        }
                                        let current_free = get_bet_for_user(
                                            bets,
                                            guess_value,
                                            &username_for_paid_plus,
                                        )
                                        .map(|b| b.num_free)
                                        .unwrap_or(0);
                                        if current_free == 0 {
                                            return true;
                                        }
                                        let Ok(game_state) = game_state_q.single() else {
                                            return true;
                                        };
                                        let score = game_state
                                            .scores
                                            .get(&username_for_paid_plus)
                                            .copied()
                                            .unwrap_or(0);
                                        get_total_paid_bets(bets, &username_for_paid_plus) >= score
                                    },
                                )
                                .with_on_release(
                                    move |mut inputs_queue: ResMut<InputsQueue>,
                                          bets_q: Query<&Bets, With<BetsActive>>| {
                                        let (current_free, current_paid) = bets_q
                                            .single()
                                            .ok()
                                            .and_then(|bets| {
                                                get_bet_for_user(
                                                    bets,
                                                    guess_value,
                                                    &username_for_paid_plus_release,
                                                )
                                            })
                                            .map(|b| (b.num_free, b.num_paid))
                                            .unwrap_or((0, 0));
                                        inputs_queue.push(ClientInput::SubmitBet {
                                            guess: guess_value,
                                            num_free: current_free,
                                            num_paid: current_paid + 1,
                                        });
                                    },
                                )
                                .bundle(),
                        ],
                    ),
                ],
            ))
            .id();
        commands.entity(row_container).add_child(box_entity);
    }

    let username_for_lock = current_username.clone();
    let username_for_lock_disabled = current_username.clone();

    commands
        .spawn((
            FlexSimple::new().bundle(),
            BettingCleanup,
            children![
                Spacer::height(Val::Px(20.0)).bundle(),
                TextSimple::p("")
                    .with_text_system(betting_question_text)
                    .bundle(),
                Spacer::height(Val::Px(15.0)).bundle(),
                TextSimple::p("")
                    .with_text_system(betting_time_remaining_text)
                    .bundle(),
                Spacer::height(Val::Px(20.0)).bundle(),
            ],
        ))
        .add_child(row_container)
        .with_children(|parent| {
            parent.spawn((Spacer::height(Val::Px(20.0)).bundle(),));
            parent.spawn(
                ButtonSimple::small("Lock Bets")
                    .with_size(Val::Px(LOCK_BUTTON_WIDTH), Val::Px(LOCK_BUTTON_HEIGHT))
                    .with_disabled_system(move |bets_active_q: Query<&BetsActive>| -> bool {
                        let Ok(bets_active) = bets_active_q.single() else {
                            return true;
                        };
                        is_user_locked(bets_active, &username_for_lock_disabled)
                    })
                    .with_on_release(
                        move |mut inputs_queue: ResMut<InputsQueue>,
                              bets_active_q: Query<&BetsActive>| {
                            if let Ok(bets_active) = bets_active_q.single() {
                                if !is_user_locked(bets_active, &username_for_lock) {
                                    inputs_queue.push(ClientInput::LockBets);
                                }
                            }
                        },
                    )
                    .bundle(),
            );
        });
}

fn update_betting() {}

fn on_exit_betting(cleanup_q: Query<Entity, With<BettingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

#[derive(Component)]
struct ReviewingCleanup;

fn is_user_continue_locked(round_cap: &RoundCap, username: &str) -> bool {
    round_cap
        .continue_locked
        .get(username)
        .copied()
        .unwrap_or(false)
}

fn on_enter_reviewing(mut commands: Commands, connection_state: Res<State<ClientConnectionState>>) {
    let ClientConnectionState::Named { username } = connection_state.get() else {
        warn!("Client not named when entering reviewing");
        return;
    };
    let current_username = username.clone();

    let username_for_delta = current_username.clone();
    let username_for_disabled = current_username.clone();
    let username_for_release = current_username.clone();

    commands.spawn((
        FlexSimple::new().bundle(),
        ReviewingCleanup,
        children![
            TextSimple::p("Round Summary").bundle(),
            Spacer::height(Val::Px(20.0)).bundle(),
            TextSimple::p("")
                .with_text_system(move |round_cap_q: Query<&RoundCap>| -> String {
                    let Ok(round_cap) = round_cap_q.single() else {
                        return "Delta: ?".to_string();
                    };
                    let delta = round_cap
                        .delta_this_round
                        .get(&username_for_delta)
                        .copied()
                        .unwrap_or(0);
                    let delta_str = if delta >= 0 {
                        format!("+{}", delta)
                    } else {
                        format!("{}", delta)
                    };
                    format!("Delta: {}", delta_str)
                })
                .bundle(),
            Spacer::height(Val::Px(20.0)).bundle(),
            ButtonSimple::small("Continue")
                .with_disabled_system(move |round_cap_q: Query<&RoundCap>| -> bool {
                    let Ok(round_cap) = round_cap_q.single() else {
                        return true;
                    };
                    is_user_continue_locked(round_cap, &username_for_disabled)
                })
                .with_on_release(
                    move |mut inputs_queue: ResMut<InputsQueue>, round_cap_q: Query<&RoundCap>| {
                        if let Ok(round_cap) = round_cap_q.single() {
                            if !is_user_continue_locked(round_cap, &username_for_release) {
                                inputs_queue.push(ClientInput::VoteContinue);
                            }
                        }
                    },
                )
                .bundle(),
        ],
    ));
}

fn update_reviewing(
    _connection_state: Res<State<ClientConnectionState>>,
    _round_cap_q: Query<&RoundCap>,
    _inputs_queue: ResMut<InputsQueue>,
) {
}

fn on_exit_reviewing(cleanup_q: Query<Entity, With<ReviewingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

pub fn client_game_plugin_fn(app: &mut App) {
    app.add_systems(
        OnEnter(ClientGameState::WaitingForQuestion),
        on_enter_waiting_for_question,
    );
    app.add_systems(
        OnExit(ClientGameState::WaitingForQuestion),
        on_exit_waiting_for_question,
    );

    app.add_systems(OnEnter(ClientGameState::Guessing), on_enter_guessing);
    app.add_systems(
        FixedUpdate,
        update_guessing.run_if(in_state(ClientGameState::Guessing)),
    );
    app.add_systems(OnExit(ClientGameState::Guessing), on_exit_guessing);

    app.add_systems(OnEnter(ClientGameState::Betting), on_enter_betting);
    app.add_systems(
        FixedUpdate,
        update_betting.run_if(in_state(ClientGameState::Betting)),
    );
    app.add_systems(OnExit(ClientGameState::Betting), on_exit_betting);

    app.add_systems(OnEnter(ClientGameState::Reviewing), on_enter_reviewing);
    app.add_systems(
        FixedUpdate,
        update_reviewing.run_if(in_state(ClientGameState::Reviewing)),
    );
    app.add_systems(OnExit(ClientGameState::Reviewing), on_exit_reviewing);
}
