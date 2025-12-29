use std::time::Duration;

use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::{Connected, LinkOf};

use crate::{host::host_state::HostState, server_state::temporary_start_game};

#[derive(Component)]
struct LobbyCleanup;

fn connected_count_text(connected_q: Query<Entity, (With<LinkOf>, With<Connected>)>) -> String {
    let num_connected = connected_q.iter().count();
    format!("Connected: {}", num_connected)
}

fn player_list_text(player_info_q: Query<&PlayerInfo>) -> String {
    let Ok(player_info) = player_info_q.single() else {
        return "Players:\n(no player info)".to_string();
    };

    let mut lines = vec!["Players:".to_string()];

    // Named players first
    for named in &player_info.named_players {
        let status = if named.peer_id.is_none() {
            " [DISCONNECTED]"
        } else {
            ""
        };
        lines.push(format!("  - {}{}", named.username, status));
    }

    // Then unnamed players
    for _ in &player_info.unnamed_players {
        lines.push("  - (unnamed)".to_string());
    }

    lines.join("\n")
}

fn is_start_disabled(player_info_q: Query<&PlayerInfo>) -> bool {
    let Ok(player_info) = player_info_q.single() else {
        return true;
    };

    let connected_named_count = player_info
        .named_players
        .iter()
        .filter(|named| named.peer_id.is_some())
        .count();

    connected_named_count < 2
}

fn on_enter_lobby(mut commands: Commands) {
    commands.spawn((
        FlexSimple::new().bundle(),
        LobbyCleanup,
        children![
            TextSimple::p("")
                .with_text_system(connected_count_text)
                .bundle(),
            Spacer::height(Val::Px(10.0)).bundle(),
            TextSimple::p("")
                .with_text_system(player_list_text)
                .bundle(),
            Spacer::height(Val::Px(20.0)).bundle(),
            ButtonSimple::medium("START")
                .with_on_release(temporary_start_game)
                .with_disabled_system(is_start_disabled)
                .bundle()
        ],
    ));
}

fn update_lobby(player_info_q: Query<&PlayerInfo>, mut commands: Commands) {
    let Ok(player_info) = player_info_q.single() else {
        return;
    };
    let num_fully_ready = player_info
        .named_players
        .iter()
        .filter(|named| named.peer_id.is_some())
        .count();
    // HACK: just auto start it
    if num_fully_ready >= 2 {
        commands.run_system_cached(temporary_start_game);
    }
}

fn on_exit_lobby(cleanup_q: Query<Entity, With<LobbyCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

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
        "Generating question{}",
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

fn guessing_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    let Ok(question) = question_q.single() else {
        return "(no question)".to_string();
    };
    question.question.clone()
}

fn guessing_players_text(
    question_q: Query<&Question, With<QuestionActive>>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(question) = question_q.single() else {
        return "Guessing:".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "Guessing:".to_string();
    };

    let mut lines = vec!["Guessing:".to_string()];
    for named in &player_info.named_players {
        if !question.guesses.contains_key(&named.username) {
            lines.push(format!("  - {}", named.username));
        }
    }
    lines.join("\n")
}

fn submitted_players_text(
    question_q: Query<&Question, With<QuestionActive>>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(question) = question_q.single() else {
        return "Submitted:".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "Submitted:".to_string();
    };

    let mut lines = vec!["Submitted:".to_string()];
    for named in &player_info.named_players {
        if question.guesses.contains_key(&named.username) {
            lines.push(format!("  - {}", named.username));
        }
    }
    lines.join("\n")
}

fn on_enter_guessing(mut commands: Commands) {
    commands.spawn((
        FlexSimple::new().bundle(),
        GuessingCleanup,
        children![
            // Top half: question
            (
                FlexSimple::new()
                    .with_size(Val::Percent(100.0), Val::Percent(50.0))
                    .bundle(),
                children![
                    TextSimple::p("")
                        .with_text_system(guessing_question_text)
                        .bundle(),
                ],
            ),
            // Bottom half: two columns
            (
                FlexSimple::new()
                    .with_direction(FlexDirection::Row)
                    .with_size(Val::Percent(100.0), Val::Percent(50.0))
                    .bundle(),
                children![
                    // Left column: Guessing
                    (
                        FlexSimple::new()
                            .with_size(Val::Percent(50.0), Val::Percent(100.0))
                            .bundle(),
                        children![
                            TextSimple::p("")
                                .with_text_system(guessing_players_text)
                                .bundle(),
                        ],
                    ),
                    // Right column: Submitted
                    (
                        FlexSimple::new()
                            .with_size(Val::Percent(50.0), Val::Percent(100.0))
                            .bundle(),
                        children![
                            TextSimple::p("")
                                .with_text_system(submitted_players_text)
                                .bundle(),
                        ],
                    ),
                ],
            ),
        ],
    ));
}

fn on_exit_guessing(cleanup_q: Query<Entity, With<GuessingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

#[derive(Component)]
struct BettingCleanup;

fn betting_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| q.question.clone())
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

fn betting_bets_locked_text(
    bets_active_q: Query<&BetsActive>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(active) = bets_active_q.single() else {
        return "Bets Locked:".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "Bets Locked:".to_string();
    };

    let locked_names: Vec<&str> = active
        .bets_locked
        .iter()
        .filter(|(_, locked)| **locked)
        .map(|(name, _)| name.as_str())
        .collect();

    let total = player_info.named_players.len();
    let locked_count = locked_names.len();

    if locked_names.is_empty() {
        format!("Bets Locked: (none) ({}/{})", locked_count, total)
    } else {
        format!(
            "Bets Locked: {} ({}/{})",
            locked_names.join(", "),
            locked_count,
            total
        )
    }
}

fn on_enter_betting(mut commands: Commands, question_q: Query<&Question, With<QuestionActive>>) {
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
        let box_entity = commands
            .spawn((
                Node {
                    width: Val::Percent(box_width_percent),
                    height: Val::Auto,
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
                BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
                children![
                    TextSimple::p("")
                        .with_font_size(14.0)
                        .with_size(Val::Percent(100.0), Val::Auto)
                        .with_text_system(
                            move |question_q: Query<&Question, With<QuestionActive>>,
                                  bets_q: Query<&Bets, With<BetsActive>>|
                                  -> String {
                                let mut lines = Vec::new();

                                if guess_value == 0 {
                                    lines.push("LOWBALL".to_string());
                                } else if let Ok(question) = question_q.single() {
                                    let owners: Vec<&str> = question
                                        .guesses
                                        .iter()
                                        .filter(|(_, v)| **v == guess_value)
                                        .map(|(name, _)| name.as_str())
                                        .collect();
                                    lines.push(format!("Owner(s): {}", owners.join(", ")));
                                }

                                lines.push(String::new());
                                lines.push(format!("Guess: {}", guess_value));
                                lines.push(String::new());
                                lines.push("Bets:".to_string());

                                if let Ok(bets) = bets_q.single() {
                                    if let Some(bet_list) = bets.bets.get(&guess_value) {
                                        for bet in bet_list {
                                            let amount = bet.num_free + bet.num_paid;
                                            lines.push(format!("  - {}: {}", bet.owner, amount));
                                        }
                                    }
                                }

                                lines.join("\n")
                            }
                        )
                        .bundle()
                ],
            ))
            .id();
        commands.entity(row_container).add_child(box_entity);
    }

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
                Spacer::height(Val::Px(10.0)).bundle(),
                TextSimple::p("")
                    .with_text_system(betting_bets_locked_text)
                    .bundle(),
                Spacer::height(Val::Px(20.0)).bundle(),
            ],
        ))
        .add_child(row_container);
}

fn on_exit_betting(cleanup_q: Query<Entity, With<BettingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

pub fn host_game_plugin_fn(app: &mut App) {
    app.add_systems(OnEnter(HostState::Lobby), on_enter_lobby);
    app.add_systems(
        FixedUpdate,
        (update_lobby,).chain().run_if(in_state(HostState::Lobby)),
    );
    app.add_systems(OnExit(HostState::Lobby), on_exit_lobby);

    app.add_systems(
        OnEnter(HostState::WaitingForQuestion),
        on_enter_waiting_for_question,
    );
    app.add_systems(
        OnExit(HostState::WaitingForQuestion),
        on_exit_waiting_for_question,
    );

    app.add_systems(OnEnter(HostState::Guessing), on_enter_guessing);
    app.add_systems(OnExit(HostState::Guessing), on_exit_guessing);

    app.add_systems(OnEnter(HostState::Betting), on_enter_betting);
    app.add_systems(OnExit(HostState::Betting), on_exit_betting);
}
