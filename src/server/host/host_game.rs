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

    let mut named_players: Vec<_> = player_info.named_players.iter().collect();
    named_players.sort_by_key(|n| &n.username);
    for named in named_players {
        let status = if named.peer_id.is_none() {
            " [DISCONNECTED]"
        } else {
            ""
        };
        lines.push(format!("  - {}{}", named.username, status));
    }

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
    let mut guessing: Vec<&str> = player_info
        .named_players
        .iter()
        .filter(|n| !question.guesses.contains_key(&n.username))
        .map(|n| n.username.as_str())
        .collect();
    guessing.sort();
    for name in guessing {
        lines.push(format!("  - {}", name));
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
    let mut submitted: Vec<&str> = player_info
        .named_players
        .iter()
        .filter(|n| question.guesses.contains_key(&n.username))
        .map(|n| n.username.as_str())
        .collect();
    submitted.sort();
    for name in submitted {
        lines.push(format!("  - {}", name));
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

    let mut locked_names: Vec<&str> = active
        .bets_locked
        .iter()
        .filter(|(_, locked)| **locked)
        .map(|(name, _)| name.as_str())
        .collect();
    locked_names.sort();

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
                                    let mut owners: Vec<&str> = question
                                        .guesses
                                        .iter()
                                        .filter(|(_, v)| **v == guess_value)
                                        .map(|(name, _)| name.as_str())
                                        .collect();
                                    owners.sort();
                                    lines.push(format!("Owner(s): {}", owners.join(", ")));
                                }

                                lines.push(String::new());
                                lines.push(format!("Guess: {}", guess_value));
                                lines.push(String::new());
                                lines.push("Bets:".to_string());

                                if let Ok(bets) = bets_q.single() {
                                    if let Some(bet_list) = bets.bets.get(&guess_value) {
                                        let mut sorted_bets: Vec<_> = bet_list.iter().collect();
                                        sorted_bets.sort_by_key(|b| &b.owner);
                                        for bet in sorted_bets {
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

#[derive(Component)]
struct ReviewingCleanup;

fn reviewing_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| q.question.clone())
        .unwrap_or("(no question)".to_string())
}

fn reviewing_time_remaining_text(round_cap_q: Query<&RoundCap>) -> String {
    if let Ok(round_cap) = round_cap_q.single() {
        let seconds = round_cap
            .seconds_until_auto_continue
            .map(|s| s.max(0.0))
            .unwrap_or(0.0);
        format!("Next round in: {:.1}s", seconds)
    } else {
        "Next round in: 0.0s".to_string()
    }
}

fn reviewing_continue_votes_text(
    round_cap_q: Query<&RoundCap>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(round_cap) = round_cap_q.single() else {
        return "Continue:".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "Continue:".to_string();
    };

    let mut voted_names: Vec<&str> = round_cap
        .continue_locked
        .iter()
        .filter(|(_, voted)| **voted)
        .map(|(name, _)| name.as_str())
        .collect();
    voted_names.sort();

    let total = player_info.named_players.len();
    let voted_count = voted_names.len();

    if voted_names.is_empty() {
        format!("Continue: (none) ({}/{})", voted_count, total)
    } else {
        format!(
            "Continue: {} ({}/{})",
            voted_names.join(", "),
            voted_count,
            total
        )
    }
}

fn reviewing_correct_answer_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| q.answer.to_string())
        .unwrap_or("?".to_string())
}

fn get_winning_guess(question: &Question) -> u32 {
    let answer = question.answer;
    question
        .guesses
        .values()
        .filter(|&&g| g <= answer)
        .copied()
        .max()
        .unwrap_or(0)
}

fn reviewing_winning_bet_text(
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<&Bets, With<BetsActive>>,
) -> String {
    let Ok(question) = question_q.single() else {
        return "No question".to_string();
    };
    let Ok(bets) = bets_q.single() else {
        return "No bets".to_string();
    };

    let winning_guess = get_winning_guess(question);
    let mut lines = Vec::new();

    if winning_guess == 0 {
        lines.push("LOWBALL".to_string());
    } else {
        let mut owners: Vec<&str> = question
            .guesses
            .iter()
            .filter(|(_, v)| **v == winning_guess)
            .map(|(name, _)| name.as_str())
            .collect();
        owners.sort();
        lines.push(format!("Owner(s): {}", owners.join(", ")));
    }

    lines.push(String::new());
    lines.push(format!("Guess: {}", winning_guess));
    lines.push(String::new());
    lines.push("Bets:".to_string());

    let bet_list = bets.bets.get(&winning_guess);
    if bet_list.map(|b| b.is_empty()).unwrap_or(true) {
        lines.push("  (none)".to_string());
    } else if let Some(bet_list) = bet_list {
        let mut sorted_bets: Vec<_> = bet_list.iter().collect();
        sorted_bets.sort_by_key(|b| &b.owner);
        for bet in sorted_bets {
            let amount = bet.num_free + bet.num_paid;
            lines.push(format!("  - {}: {}", bet.owner, amount));
        }
    }

    lines.join("\n")
}

fn reviewing_scores_text(game_state_q: Query<&GameState>, round_cap_q: Query<&RoundCap>) -> String {
    let Ok(game_state) = game_state_q.single() else {
        return "Scores:\n(no game state)".to_string();
    };
    let Ok(round_cap) = round_cap_q.single() else {
        return "Scores:\n(no round cap)".to_string();
    };

    let mut lines = vec!["Scores:".to_string()];

    let mut scores: Vec<_> = game_state.scores.iter().collect();
    scores.sort_by(|a, b| b.1.cmp(a.1));

    for (username, &score) in scores {
        let delta = round_cap
            .delta_this_round
            .get(username)
            .copied()
            .unwrap_or(0);
        let delta_str = if delta >= 0 {
            format!("+{}", delta)
        } else {
            format!("{}", delta)
        };
        lines.push(format!("{}: {} ({})", username, score, delta_str));
    }

    lines.join("\n")
}

fn on_enter_reviewing(mut commands: Commands, game_state_q: Query<&GameState>) {
    // HACK: Log scores for testing
    if let Ok(game_state) = game_state_q.single() {
        let mut scores: Vec<_> = game_state.scores.iter().collect();
        scores.sort_by(|a, b| b.1.cmp(a.1));
        let score_strs: Vec<String> = scores
            .iter()
            .map(|(name, score)| format!("{}: {}", name, score))
            .collect();
        info!(
            "mork - Round {} Scores: {}",
            game_state.round,
            score_strs.join(", ")
        );
    }
    commands.spawn((
        FlexSimple::new().bundle(),
        ReviewingCleanup,
        children![
            (
                FlexSimple::new()
                    .with_size(Val::Percent(100.0), Val::Percent(15.0))
                    .bundle(),
                children![
                    TextSimple::p("")
                        .with_text_system(reviewing_question_text)
                        .bundle(),
                    Spacer::height(Val::Px(10.0)).bundle(),
                    TextSimple::p("")
                        .with_text_system(reviewing_time_remaining_text)
                        .bundle(),
                    Spacer::height(Val::Px(5.0)).bundle(),
                    TextSimple::p("")
                        .with_text_system(reviewing_continue_votes_text)
                        .bundle(),
                ],
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(85.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(20.0),
                    padding: UiRect::horizontal(Val::Px(20.0)),
                    ..default()
                },
                children![
                    (
                        Node {
                            width: Val::Percent(33.33),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            padding: UiRect::all(Val::Px(15.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
                        BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
                        children![
                            TextSimple::p("Correct Answer")
                                .with_font_size(18.0)
                                .bundle(),
                            Spacer::height(Val::Px(20.0)).bundle(),
                            TextSimple::p("")
                                .with_font_size(72.0)
                                .with_text_system(reviewing_correct_answer_text)
                                .bundle(),
                        ],
                    ),
                    (
                        Node {
                            width: Val::Percent(33.33),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            padding: UiRect::all(Val::Px(15.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
                        BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
                        children![
                            TextSimple::p("Winning Bet").with_font_size(18.0).bundle(),
                            Spacer::height(Val::Px(15.0)).bundle(),
                            TextSimple::p("")
                                .with_font_size(14.0)
                                .with_text_system(reviewing_winning_bet_text)
                                .bundle(),
                        ],
                    ),
                    (
                        Node {
                            width: Val::Percent(33.33),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            padding: UiRect::all(Val::Px(15.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
                        BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
                        children![
                            TextSimple::p("")
                                .with_font_size(14.0)
                                .with_text_system(reviewing_scores_text)
                                .bundle(),
                        ],
                    ),
                ],
            ),
        ],
    ));
}

fn on_exit_reviewing(cleanup_q: Query<Entity, With<ReviewingCleanup>>, mut commands: Commands) {
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

    app.add_systems(OnEnter(HostState::Reviewing), on_enter_reviewing);
    app.add_systems(OnExit(HostState::Reviewing), on_exit_reviewing);
}
