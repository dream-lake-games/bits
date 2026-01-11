//! Host game UI - read-only spectator view for the TV display.
//!
//! The host sees the game state but cannot interact (no inputs).
//! This is meant to be projected on a shared screen for all players to watch.

use bevy::prelude::*;
use bits::host::{WaitingForQuestionScreen, waiting_for_question_plugin_fn};
use bits::prelude::*;

use crate::client_state::{ClientGameState, ClientRoleState};

const TEXT_SPEED: f32 = 0.02;

// =============================================================================
// Waiting For Question
// =============================================================================

fn on_enter_waiting_for_question(mut commands: Commands) {
    commands.spawn((
        Name::new("WaitingForQuestionScreen"),
        WaitingForQuestionScreen,
    ));
}

fn on_exit_waiting_for_question(
    cleanup_q: Query<Entity, With<WaitingForQuestionScreen>>,
    mut commands: Commands,
) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Guessing Phase
// =============================================================================

#[derive(Component)]
struct GuessingCleanup;

fn format_question_with_units(question: &Question) -> String {
    match &question.units {
        Some(units) => format!(
            "{} IN {}",
            question.question.to_uppercase(),
            units.to_uppercase()
        ),
        None => question.question.to_uppercase(),
    }
}

fn guessing_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| format_question_with_units(q))
        .unwrap_or("NO QUESTION".to_string())
}

fn guessing_status_text(
    question_q: Query<&Question, With<QuestionActive>>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(question) = question_q.single() else {
        return "STATUS".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "STATUS".to_string();
    };

    let mut waiting: Vec<&str> = player_info
        .named_players
        .iter()
        .filter(|n| !question.guesses.contains_key(&n.username))
        .map(|n| n.username.as_str())
        .collect();
    waiting.sort();

    let mut submitted: Vec<&str> = player_info
        .named_players
        .iter()
        .filter(|n| question.guesses.contains_key(&n.username))
        .map(|n| n.username.as_str())
        .collect();
    submitted.sort();

    let mut lines = Vec::new();

    if !waiting.is_empty() {
        lines.push(format!("WAITING {}", waiting.join(" ")));
    }
    if !submitted.is_empty() {
        lines.push(format!("SUBMITTED {}", submitted.join(" ")));
    }

    lines.join(" ")
}

fn guessing_timer_text(question_q: Query<&QuestionActive>) -> String {
    if let Ok(active) = question_q.single() {
        let seconds = active
            .guess_seconds_remaining
            .map(|s| s.max(0.0))
            .unwrap_or(0.0);
        format!("TIME {:.0}", seconds)
    } else {
        "TIME 0".to_string()
    }
}

fn on_enter_guessing(mut commands: Commands) {
    // Question at top
    commands.spawn((
        Name::new("Question"),
        GuessingCleanup,
        AnimatedText::new("QUESTION", UVec2::new(768, 128), TEXT_SPEED)
            .with_text_system(guessing_question_text),
        Transform::from_xyz(0.0, 150.0, 0.0),
        Visibility::Inherited,
    ));

    // Timer
    commands.spawn((
        Name::new("Timer"),
        GuessingCleanup,
        AnimatedText::new("TIME", UVec2::new(256, 64), TEXT_SPEED)
            .with_text_system(guessing_timer_text),
        Transform::from_xyz(0.0, 50.0, 0.0),
        Visibility::Inherited,
    ));

    // Status (who's waiting/submitted)
    commands.spawn((
        Name::new("Status"),
        GuessingCleanup,
        AnimatedText::new("STATUS", UVec2::new(640, 128), TEXT_SPEED)
            .with_text_system(guessing_status_text),
        Transform::from_xyz(0.0, -80.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_guessing(cleanup_q: Query<Entity, With<GuessingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Betting Phase
// =============================================================================

#[derive(Component)]
struct BettingCleanup;

fn betting_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| format_question_with_units(q))
        .unwrap_or("NO QUESTION".to_string())
}

fn betting_timer_text(bets_active_q: Query<&BetsActive>) -> String {
    if let Ok(active) = bets_active_q.single() {
        let seconds = active
            .bets_seconds_remaining
            .map(|s| s.max(0.0))
            .unwrap_or(0.0);
        format!("TIME {:.0}", seconds)
    } else {
        "TIME 0".to_string()
    }
}

fn betting_locked_text(
    bets_active_q: Query<&BetsActive>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(active) = bets_active_q.single() else {
        return "LOCKED".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "LOCKED".to_string();
    };

    let mut locked: Vec<&str> = active
        .bets_locked
        .iter()
        .filter(|(_, locked)| **locked)
        .map(|(name, _)| name.as_str())
        .collect();
    locked.sort();

    let total = player_info.named_players.len();
    let locked_count = locked.len();

    if locked.is_empty() {
        format!("LOCKED 0 OF {}", total)
    } else {
        format!("LOCKED {} {} OF {}", locked.join(" "), locked_count, total)
    }
}

fn betting_guesses_text(
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<&Bets, With<BetsActive>>,
) -> String {
    let Ok(question) = question_q.single() else {
        return "GUESSES".to_string();
    };
    let bets = bets_q.single().ok();

    let mut guess_values: Vec<u32> = question.guesses.values().cloned().collect();
    guess_values.sort();
    guess_values.dedup();
    if !guess_values.contains(&0) {
        guess_values.insert(0, 0);
    }

    let mut lines = Vec::new();

    for guess_value in guess_values {
        let header = if guess_value == 0 {
            "LOWBALL".to_string()
        } else {
            let mut owners: Vec<&str> = question
                .guesses
                .iter()
                .filter(|(_, v)| **v == guess_value)
                .map(|(name, _)| name.as_str())
                .collect();
            owners.sort();
            owners.join(" ")
        };

        let bet_info = if let Some(bets) = bets {
            if let Some(bet_list) = bets.bets.get(&guess_value) {
                let mut bet_strs: Vec<String> = bet_list
                    .iter()
                    .map(|b| format!("{} {}", b.owner, b.num_free + b.num_paid))
                    .collect();
                bet_strs.sort();
                bet_strs.join(" ")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if bet_info.is_empty() {
            lines.push(format!("{} {}", guess_value, header));
        } else {
            lines.push(format!("{} {} BETS {}", guess_value, header, bet_info));
        }
    }

    lines.join(" ")
}

fn on_enter_betting(mut commands: Commands) {
    // Question at top
    commands.spawn((
        Name::new("Question"),
        BettingCleanup,
        AnimatedText::new("QUESTION", UVec2::new(768, 96), TEXT_SPEED)
            .with_text_system(betting_question_text),
        Transform::from_xyz(0.0, 180.0, 0.0),
        Visibility::Inherited,
    ));

    // Timer
    commands.spawn((
        Name::new("Timer"),
        BettingCleanup,
        AnimatedText::new("TIME", UVec2::new(256, 64), TEXT_SPEED)
            .with_text_system(betting_timer_text),
        Transform::from_xyz(-200.0, 80.0, 0.0),
        Visibility::Inherited,
    ));

    // Locked status
    commands.spawn((
        Name::new("LockedStatus"),
        BettingCleanup,
        AnimatedText::new("LOCKED", UVec2::new(512, 64), TEXT_SPEED)
            .with_text_system(betting_locked_text),
        Transform::from_xyz(100.0, 80.0, 0.0),
        Visibility::Inherited,
    ));

    // Guesses and bets
    commands.spawn((
        Name::new("Guesses"),
        BettingCleanup,
        AnimatedText::new("GUESSES", UVec2::new(768, 192), TEXT_SPEED)
            .with_text_system(betting_guesses_text),
        Transform::from_xyz(0.0, -80.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_betting(cleanup_q: Query<Entity, With<BettingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Reviewing Phase
// =============================================================================

#[derive(Component)]
struct ReviewingCleanup;

fn reviewing_question_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| format_question_with_units(q))
        .unwrap_or("NO QUESTION".to_string())
}

fn reviewing_answer_text(question_q: Query<&Question, With<QuestionActive>>) -> String {
    question_q
        .single()
        .map(|q| format!("ANSWER {}", q.answer))
        .unwrap_or("ANSWER ?".to_string())
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

fn reviewing_winner_text(
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<&Bets, With<BetsActive>>,
) -> String {
    let Ok(question) = question_q.single() else {
        return "WINNER".to_string();
    };
    let Ok(bets) = bets_q.single() else {
        return "WINNER".to_string();
    };

    let winning_guess = get_winning_guess(question);

    let header = if winning_guess == 0 {
        "LOWBALL".to_string()
    } else {
        let mut owners: Vec<&str> = question
            .guesses
            .iter()
            .filter(|(_, v)| **v == winning_guess)
            .map(|(name, _)| name.as_str())
            .collect();
        owners.sort();
        format!("GUESS {} BY {}", winning_guess, owners.join(" "))
    };

    let bet_info = if let Some(bet_list) = bets.bets.get(&winning_guess) {
        if bet_list.is_empty() {
            "NO BETS".to_string()
        } else {
            let mut bet_strs: Vec<String> = bet_list
                .iter()
                .map(|b| format!("{} {}", b.owner, b.num_free + b.num_paid))
                .collect();
            bet_strs.sort();
            format!("BETS {}", bet_strs.join(" "))
        }
    } else {
        "NO BETS".to_string()
    };

    format!("WINNER {} {}", header, bet_info)
}

fn reviewing_scores_text(game_state_q: Query<&GameState>, round_cap_q: Query<&RoundCap>) -> String {
    let Ok(game_state) = game_state_q.single() else {
        return "SCORES".to_string();
    };
    let Ok(round_cap) = round_cap_q.single() else {
        return "SCORES".to_string();
    };

    let mut scores: Vec<_> = game_state.scores.iter().collect();
    scores.sort_by(|a, b| b.1.cmp(a.1));

    let score_strs: Vec<String> = scores
        .iter()
        .map(|(username, score)| {
            let delta = round_cap
                .delta_this_round
                .get(*username)
                .copied()
                .unwrap_or(0);
            let delta_str = if delta >= 0 {
                format!("+{}", delta)
            } else {
                format!("{}", delta)
            };
            format!("{} {} {}", username, score, delta_str)
        })
        .collect();

    format!("SCORES {}", score_strs.join(" "))
}

fn reviewing_timer_text(round_cap_q: Query<&RoundCap>) -> String {
    if let Ok(round_cap) = round_cap_q.single() {
        let seconds = round_cap
            .seconds_until_auto_continue
            .map(|s| s.max(0.0))
            .unwrap_or(0.0);
        format!("NEXT ROUND IN {:.0}", seconds)
    } else {
        "NEXT ROUND IN 0".to_string()
    }
}

fn reviewing_continue_text(
    round_cap_q: Query<&RoundCap>,
    player_info_q: Query<&PlayerInfo>,
) -> String {
    let Ok(round_cap) = round_cap_q.single() else {
        return "CONTINUE".to_string();
    };
    let Ok(player_info) = player_info_q.single() else {
        return "CONTINUE".to_string();
    };

    let mut voted: Vec<&str> = round_cap
        .continue_locked
        .iter()
        .filter(|(_, v)| **v)
        .map(|(name, _)| name.as_str())
        .collect();
    voted.sort();

    let total = player_info.named_players.len();
    let voted_count = voted.len();

    if voted.is_empty() {
        format!("CONTINUE 0 OF {}", total)
    } else {
        format!("CONTINUE {} {} OF {}", voted.join(" "), voted_count, total)
    }
}

fn on_enter_reviewing(mut commands: Commands) {
    // Question at top
    commands.spawn((
        Name::new("Question"),
        ReviewingCleanup,
        AnimatedText::new("QUESTION", UVec2::new(768, 64), TEXT_SPEED)
            .with_text_system(reviewing_question_text),
        Transform::from_xyz(0.0, 200.0, 0.0),
        Visibility::Inherited,
    ));

    // Answer - big and prominent
    commands.spawn((
        Name::new("Answer"),
        ReviewingCleanup,
        AnimatedText::new("ANSWER", UVec2::new(512, 64), TEXT_SPEED)
            .with_text_system(reviewing_answer_text),
        Transform::from_xyz(0.0, 120.0, 0.0),
        Visibility::Inherited,
    ));

    // Winner info
    commands.spawn((
        Name::new("Winner"),
        ReviewingCleanup,
        AnimatedText::new("WINNER", UVec2::new(768, 64), TEXT_SPEED)
            .with_text_system(reviewing_winner_text),
        Transform::from_xyz(0.0, 40.0, 0.0),
        Visibility::Inherited,
    ));

    // Scores
    commands.spawn((
        Name::new("Scores"),
        ReviewingCleanup,
        AnimatedText::new("SCORES", UVec2::new(768, 96), TEXT_SPEED)
            .with_text_system(reviewing_scores_text),
        Transform::from_xyz(0.0, -60.0, 0.0),
        Visibility::Inherited,
    ));

    // Timer
    commands.spawn((
        Name::new("Timer"),
        ReviewingCleanup,
        AnimatedText::new("NEXT ROUND", UVec2::new(512, 64), TEXT_SPEED)
            .with_text_system(reviewing_timer_text),
        Transform::from_xyz(0.0, -160.0, 0.0),
        Visibility::Inherited,
    ));

    // Continue votes
    commands.spawn((
        Name::new("ContinueVotes"),
        ReviewingCleanup,
        AnimatedText::new("CONTINUE", UVec2::new(512, 64), TEXT_SPEED)
            .with_text_system(reviewing_continue_text),
        Transform::from_xyz(0.0, -220.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_reviewing(cleanup_q: Query<Entity, With<ReviewingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Plugin
// =============================================================================

pub fn client_game_host_plugin_fn(app: &mut App) {
    app.add_plugins(waiting_for_question_plugin_fn);

    // Waiting for question
    app.add_systems(
        OnEnter(ClientGameState::WaitingForQuestion),
        on_enter_waiting_for_question.run_if(in_state(ClientRoleState::Host)),
    );
    app.add_systems(
        OnExit(ClientGameState::WaitingForQuestion),
        on_exit_waiting_for_question.run_if(in_state(ClientRoleState::Host)),
    );

    // Guessing
    app.add_systems(
        OnEnter(ClientGameState::Guessing),
        on_enter_guessing.run_if(in_state(ClientRoleState::Host)),
    );
    app.add_systems(
        OnExit(ClientGameState::Guessing),
        on_exit_guessing.run_if(in_state(ClientRoleState::Host)),
    );

    // Betting
    app.add_systems(
        OnEnter(ClientGameState::Betting),
        on_enter_betting.run_if(in_state(ClientRoleState::Host)),
    );
    app.add_systems(
        OnExit(ClientGameState::Betting),
        on_exit_betting.run_if(in_state(ClientRoleState::Host)),
    );

    // Reviewing
    app.add_systems(
        OnEnter(ClientGameState::Reviewing),
        on_enter_reviewing.run_if(in_state(ClientRoleState::Host)),
    );
    app.add_systems(
        OnExit(ClientGameState::Reviewing),
        on_exit_reviewing.run_if(in_state(ClientRoleState::Host)),
    );
}
