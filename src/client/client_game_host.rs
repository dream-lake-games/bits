//! Host game UI - read-only spectator view for the TV display.
//!
//! The host sees the game state but cannot interact (no inputs).
//! This is meant to be projected on a shared screen for all players to watch.

use bevy::prelude::*;
use bits::host::{
    BetEntry, BettingScreen, GuessEntry, GuessingScreen, ReviewingScreen, ScoreEntry,
    WaitingForQuestionScreen, betting_plugin_fn, guessing_plugin_fn, reviewing_plugin_fn,
    waiting_for_question_plugin_fn,
};
use bits::prelude::*;

use crate::client_state::{ClientGameState, ClientRoleState};

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
    q: Query<Entity, With<WaitingForQuestionScreen>>,
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
        Name::new("GuessingScreen"),
        GuessingScreen::new("Loading..."),
    ));
}

fn on_exit_guessing(q: Query<Entity, With<GuessingScreen>>, mut commands: Commands) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}

fn update_guessing_from_game_state(
    mut screen_q: Query<&mut GuessingScreen>,
    question_q: Query<(&Question, &QuestionActive)>,
    player_info_q: Query<&PlayerInfo>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    let Ok((question, active)) = question_q.single() else {
        return;
    };
    let Ok(player_info) = player_info_q.single() else {
        return;
    };

    screen.question = question.question.clone();
    screen.units = question.units.clone();
    screen.seconds_remaining = active.guess_seconds_remaining;

    screen.players_submitted = question.guesses.keys().cloned().collect();
    screen.players_waiting = player_info
        .named_players
        .iter()
        .filter(|p| !question.guesses.contains_key(&p.username))
        .map(|p| p.username.clone())
        .collect();
}

// =============================================================================
// Betting Phase
// =============================================================================

fn on_enter_betting(mut commands: Commands) {
    commands.spawn((Name::new("BettingScreen"), BettingScreen::new("Loading...")));
}

fn on_exit_betting(q: Query<Entity, With<BettingScreen>>, mut commands: Commands) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}

fn build_guess_entries(
    question: &Question,
    bets: &Bets,
    all_players: &[String],
) -> Vec<GuessEntry> {
    let mut values: Vec<u32> = question.guesses.values().copied().collect();
    values.push(0);
    values.sort();
    values.dedup();

    values
        .iter()
        .map(|&value| {
            let guessers: Vec<String> = question
                .guesses
                .iter()
                .filter(|(_, v)| **v == value)
                .map(|(name, _)| name.clone())
                .collect();

            let mut bet_entries: Vec<BetEntry> = all_players
                .iter()
                .map(|player| {
                    let amount = bets
                        .bets
                        .get(&value)
                        .and_then(|list| list.iter().find(|b| &b.owner == player))
                        .map(|b| b.num_free + b.num_paid)
                        .unwrap_or(0);
                    BetEntry {
                        player: player.clone(),
                        amount,
                    }
                })
                .collect();
            bet_entries.sort_by(|a, b| a.player.cmp(&b.player));

            GuessEntry {
                value,
                guessers,
                bets: bet_entries,
            }
        })
        .collect()
}

fn update_betting_from_game_state(
    mut screen_q: Query<&mut BettingScreen>,
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<(&Bets, &BetsActive)>,
    player_info_q: Query<&PlayerInfo>,
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
    let Ok(player_info) = player_info_q.single() else {
        return;
    };

    screen.question = question.question.clone();
    screen.units = question.units.clone();
    screen.seconds_remaining = active.bets_seconds_remaining;

    screen.all_players = player_info
        .named_players
        .iter()
        .map(|p| p.username.clone())
        .collect();

    screen.players_locked = active
        .bets_locked
        .iter()
        .filter(|(_, locked)| **locked)
        .map(|(name, _)| name.clone())
        .collect();

    screen.guesses = build_guess_entries(question, bets, &screen.all_players);
}

// =============================================================================
// Reviewing Phase
// =============================================================================

fn on_enter_reviewing(mut commands: Commands) {
    commands.spawn((
        Name::new("ReviewingScreen"),
        ReviewingScreen::new("Loading...", 0),
    ));
}

fn on_exit_reviewing(q: Query<Entity, With<ReviewingScreen>>, mut commands: Commands) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}

fn update_reviewing_from_game_state(
    mut screen_q: Query<&mut ReviewingScreen>,
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<&Bets, With<BetsActive>>,
    round_cap_q: Query<&RoundCap>,
    game_state_q: Query<&GameState>,
) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    let Ok(question) = question_q.single() else {
        return;
    };
    let Ok(bets) = bets_q.single() else {
        return;
    };
    let Ok(round_cap) = round_cap_q.single() else {
        return;
    };
    let Ok(game_state) = game_state_q.single() else {
        return;
    };

    screen.question = question.question.clone();
    screen.units = question.units.clone();
    screen.answer = question.answer;
    screen.seconds_until_continue = round_cap.seconds_until_auto_continue;

    let winning_guess = question
        .guesses
        .values()
        .filter(|&&g| g <= question.answer)
        .copied()
        .max()
        .unwrap_or(0);

    screen.winning_guess = winning_guess;
    screen.winning_guessers = question
        .guesses
        .iter()
        .filter(|(_, v)| **v == winning_guess)
        .map(|(name, _)| name.clone())
        .collect();

    screen.winning_bets = bets
        .bets
        .get(&winning_guess)
        .map(|list| {
            list.iter()
                .map(|b| BetEntry {
                    player: b.owner.clone(),
                    amount: b.num_free + b.num_paid,
                })
                .collect()
        })
        .unwrap_or_default();

    screen.scores = game_state
        .scores
        .iter()
        .map(|(name, &score)| {
            let delta = round_cap.delta_this_round.get(name).copied().unwrap_or(0);
            ScoreEntry {
                player: name.clone(),
                score: score as i32,
                delta,
            }
        })
        .collect();

    screen.players_voted = round_cap
        .continue_locked
        .iter()
        .filter(|(_, v)| **v)
        .map(|(name, _)| name.clone())
        .collect();
}

// =============================================================================
// Plugin
// =============================================================================

pub fn client_game_host_plugin_fn(app: &mut App) {
    app.add_plugins((
        waiting_for_question_plugin_fn,
        guessing_plugin_fn,
        betting_plugin_fn,
        reviewing_plugin_fn,
    ));

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
    app.add_systems(
        Update,
        update_guessing_from_game_state
            .run_if(in_state(ClientGameState::Guessing))
            .run_if(in_state(ClientRoleState::Host)),
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
    app.add_systems(
        Update,
        update_betting_from_game_state
            .run_if(in_state(ClientGameState::Betting))
            .run_if(in_state(ClientRoleState::Host)),
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
    app.add_systems(
        Update,
        update_reviewing_from_game_state
            .run_if(in_state(ClientGameState::Reviewing))
            .run_if(in_state(ClientRoleState::Host)),
    );
}
