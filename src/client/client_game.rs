//! Player game UI - interactive phone UI for players.
//!
//! Players see game state and can interact (submit guesses, place bets, vote continue).
//! This runs on the player's phone/device.

use bevy::prelude::*;
use bits::player::{
    BettingGuessDisplay, PlayerBettingScreen, PlayerGuessingScreen, PlayerReviewingScreen,
    PlayerWaitingScreen, player_betting_plugin_fn, player_guessing_plugin_fn,
    player_reviewing_plugin_fn, player_waiting_plugin_fn,
};
use bits::prelude::*;

use crate::{
    client_simple::InputsQueue,
    client_state::{ClientConnectionState, ClientGameState, ClientRoleState},
};

// =============================================================================
// Waiting For Question
// =============================================================================

fn on_enter_waiting_for_question(mut commands: Commands) {
    commands.spawn((Name::new("PlayerWaitingScreen"), PlayerWaitingScreen));
}

fn on_exit_waiting_for_question(
    q: Query<Entity, With<PlayerWaitingScreen>>,
    mut commands: Commands,
) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Guessing Phase
// =============================================================================

fn on_enter_guessing(mut commands: Commands) {
    commands.spawn((
        Name::new("PlayerGuessingScreen"),
        PlayerGuessingScreen::new("Loading..."),
    ));
}

fn on_exit_guessing(q: Query<Entity, With<PlayerGuessingScreen>>, mut commands: Commands) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}

fn update_guessing_from_game_state(
    mut screen_q: Query<&mut PlayerGuessingScreen>,
    question_q: Query<(&Question, &QuestionActive)>,
    connection_state: Res<State<ClientConnectionState>>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    let Ok((question, active)) = question_q.single() else {
        return;
    };

    screen.question = question.question.clone();
    screen.units = question.units.clone();
    screen.seconds_remaining = active.guess_seconds_remaining;

    if let ClientConnectionState::Named { username } = connection_state.get() {
        screen.already_submitted = question.guesses.get(username).copied();
    }
}

fn drain_guessing_outputs(
    mut screen_q: Query<&mut PlayerGuessingScreen>,
    mut inputs_queue: ResMut<InputsQueue>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };

    if screen.submit_requested {
        if let Ok(guess) = screen.pending_guess.parse::<u32>() {
            inputs_queue.push(ClientInput::SubmitGuess { guess });
        }
        screen.submit_requested = false;
        screen.pending_guess.clear();
    }
}

// =============================================================================
// Betting Phase
// =============================================================================

#[derive(Component)]
struct BettingPlaceholder;

fn on_enter_betting(mut commands: Commands) {
    commands.spawn((Name::new("BettingPlaceholder"), BettingPlaceholder));
}

fn on_exit_betting(
    screen_q: Query<Entity, With<PlayerBettingScreen>>,
    placeholder_q: Query<Entity, With<BettingPlaceholder>>,
    mut commands: Commands,
) {
    for ent in &screen_q {
        commands.entity(ent).despawn();
    }
    for ent in &placeholder_q {
        commands.entity(ent).despawn();
    }
}

/// Waits for betting data to arrive from server, then spawns the real screen.
/// The betting screen's observer spawns buttons based on screen.guesses at add time,
/// so we must wait for Question/Bets data before spawning. This handles the race
/// condition where state transitions before server data is fully replicated.
fn spawn_betting_screen_when_ready(
    mut commands: Commands,
    placeholder_q: Query<Entity, With<BettingPlaceholder>>,
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<(&Bets, &BetsActive)>,
    game_state_q: Query<&GameState>,
    connection_state: Res<State<ClientConnectionState>>,
) {
    let Ok(placeholder) = placeholder_q.single() else {
        return;
    };
    let Ok(question) = question_q.single() else {
        return;
    };
    let Ok((bets, active)) = bets_q.single() else {
        return;
    };
    let ClientConnectionState::Named { username } = connection_state.get() else {
        return;
    };

    let guesses = build_betting_guess_displays(question, bets, username);
    if guesses.is_empty() {
        return;
    }

    // Data is ready - despawn placeholder and spawn real screen
    commands.entity(placeholder).despawn();

    let player_score = game_state_q
        .single()
        .map(|gs| gs.scores.get(username).copied().unwrap_or(0))
        .unwrap_or(0);

    commands.spawn((
        Name::new("PlayerBettingScreen"),
        PlayerBettingScreen::new(question.question.clone())
            .with_units(question.units.clone().unwrap_or_default())
            .with_seconds_remaining(active.bets_seconds_remaining.unwrap_or(0.0))
            .with_player_score(player_score)
            .with_is_locked(active.bets_locked.get(username).copied().unwrap_or(false))
            .with_guesses(guesses),
    ));
}

fn build_betting_guess_displays(
    question: &Question,
    bets: &Bets,
    username: &str,
) -> Vec<BettingGuessDisplay> {
    let mut values: Vec<u32> = question.guesses.values().copied().collect();
    values.push(0); // lowball
    values.sort();
    values.dedup();

    values
        .iter()
        .map(|&value| {
            let owners: Vec<String> = question
                .guesses
                .iter()
                .filter(|(_, v)| **v == value)
                .map(|(name, _)| name.clone())
                .collect();

            let (my_free_bets, my_paid_bets) = bets
                .bets
                .get(&value)
                .and_then(|list| list.iter().find(|b| b.owner == username))
                .map(|b| (b.num_free, b.num_paid))
                .unwrap_or((0, 0));

            BettingGuessDisplay {
                value,
                owners,
                my_free_bets,
                my_paid_bets,
            }
        })
        .collect()
}

fn update_betting_from_game_state(
    mut screen_q: Query<&mut PlayerBettingScreen>,
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<(&Bets, &BetsActive)>,
    game_state_q: Query<&GameState>,
    connection_state: Res<State<ClientConnectionState>>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    let Ok(question) = question_q.single() else {
        return;
    };
    let Ok((bets, active)) = bets_q.single() else {
        return;
    };

    let ClientConnectionState::Named { username } = connection_state.get() else {
        return;
    };

    screen.question = question.question.clone();
    screen.units = question.units.clone();
    screen.seconds_remaining = active.bets_seconds_remaining;

    screen.is_locked = active.bets_locked.get(username).copied().unwrap_or(false);

    if let Ok(game_state) = game_state_q.single() {
        screen.player_score = game_state.scores.get(username).copied().unwrap_or(0);
    }

    screen.guesses = build_betting_guess_displays(question, bets, username);
}

fn drain_betting_outputs(
    mut screen_q: Query<&mut PlayerBettingScreen>,
    mut inputs_queue: ResMut<InputsQueue>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };

    if let Some(pending_bet) = screen.pending_bet.take() {
        inputs_queue.push(ClientInput::SubmitBet {
            guess: pending_bet.guess,
            num_free: pending_bet.num_free,
            num_paid: pending_bet.num_paid,
        });
    }

    if screen.lock_requested {
        inputs_queue.push(ClientInput::LockBets);
        screen.lock_requested = false;
    }
}

// =============================================================================
// Reviewing Phase
// =============================================================================

fn on_enter_reviewing(mut commands: Commands) {
    commands.spawn((
        Name::new("PlayerReviewingScreen"),
        PlayerReviewingScreen::new(0),
    ));
}

fn on_exit_reviewing(q: Query<Entity, With<PlayerReviewingScreen>>, mut commands: Commands) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}

fn update_reviewing_from_game_state(
    mut screen_q: Query<&mut PlayerReviewingScreen>,
    round_cap_q: Query<&RoundCap>,
    game_state_q: Query<&GameState>,
    connection_state: Res<State<ClientConnectionState>>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    let Ok(round_cap) = round_cap_q.single() else {
        return;
    };

    let ClientConnectionState::Named { username } = connection_state.get() else {
        return;
    };

    screen.delta_this_round = round_cap
        .delta_this_round
        .get(username)
        .copied()
        .unwrap_or(0);

    if let Ok(game_state) = game_state_q.single() {
        screen.total_score = game_state.scores.get(username).copied().unwrap_or(0);
    }

    screen.already_voted = round_cap
        .continue_locked
        .get(username)
        .copied()
        .unwrap_or(false);

    screen.seconds_until_auto = round_cap.seconds_until_auto_continue;
}

fn drain_reviewing_outputs(
    mut screen_q: Query<&mut PlayerReviewingScreen>,
    mut inputs_queue: ResMut<InputsQueue>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };

    if screen.continue_requested {
        inputs_queue.push(ClientInput::VoteContinue);
        screen.continue_requested = false;
    }
}

// =============================================================================
// Plugin
// =============================================================================

pub fn client_game_plugin_fn(app: &mut App) {
    app.add_plugins((
        player_waiting_plugin_fn,
        player_guessing_plugin_fn,
        player_betting_plugin_fn,
        player_reviewing_plugin_fn,
    ));

    // Waiting for question
    app.add_systems(
        OnEnter(ClientGameState::WaitingForQuestion),
        on_enter_waiting_for_question.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        OnExit(ClientGameState::WaitingForQuestion),
        on_exit_waiting_for_question.run_if(in_state(ClientRoleState::Player)),
    );

    // Guessing
    app.add_systems(
        OnEnter(ClientGameState::Guessing),
        on_enter_guessing.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        OnExit(ClientGameState::Guessing),
        on_exit_guessing.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        Update,
        (update_guessing_from_game_state, drain_guessing_outputs)
            .chain()
            .run_if(in_state(ClientGameState::Guessing))
            .run_if(in_state(ClientRoleState::Player)),
    );

    // Betting
    app.add_systems(
        OnEnter(ClientGameState::Betting),
        on_enter_betting.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        OnExit(ClientGameState::Betting),
        on_exit_betting.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        Update,
        (
            spawn_betting_screen_when_ready,
            update_betting_from_game_state,
            drain_betting_outputs,
        )
            .chain()
            .run_if(in_state(ClientGameState::Betting))
            .run_if(in_state(ClientRoleState::Player)),
    );

    // Reviewing
    app.add_systems(
        OnEnter(ClientGameState::Reviewing),
        on_enter_reviewing.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        OnExit(ClientGameState::Reviewing),
        on_exit_reviewing.run_if(in_state(ClientRoleState::Player)),
    );
    app.add_systems(
        Update,
        (update_reviewing_from_game_state, drain_reviewing_outputs)
            .chain()
            .run_if(in_state(ClientGameState::Reviewing))
            .run_if(in_state(ClientRoleState::Player)),
    );
}
