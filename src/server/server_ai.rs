use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::Task;
use bevy::tasks::futures_lite::future;
use bits::prelude::*;

const MAX_AI_ATTEMPTS: usize = 3;

#[derive(Component)]
pub struct AIState {
    pub guess_attempts: HashMap<Username, Vec<AIGuessAttempt>>,
    pub bet_attempts: HashMap<Username, Vec<AIBetAttempt>>,
}

#[derive(Clone)]
pub struct AIGuessAttempt {
    pub guess: AIGuess,
    pub invalid_reason: Option<AIInvalidGuessReason>,
}

#[derive(Clone)]
pub struct AIBetAttempt {
    pub bets: AIBets,
    pub invalid_reason: Option<AIInvalidBetsReason>,
}

#[derive(Resource, Default)]
pub struct AITasksInFlight {
    pub guess_tasks: HashMap<Username, Task<anyhow::Result<AIGuess>>>,
    pub bet_tasks: HashMap<Username, Task<anyhow::Result<AIBets>>>,
}

pub fn setup_ai_clients(mut commands: Commands) {
    let openai = async_openai::Client::new();
    let exa = std::env::var("EXA_API_KEY")
        .ok()
        .map(bits::prelude::ExaClient::new);
    commands.insert_resource(AIClients { openai, exa });
    commands.init_resource::<AITasksInFlight>();
}

pub fn spawn_ai_state(
    question_q: Query<Entity, (With<QuestionActive>, Without<AIState>)>,
    player_info: Single<&PlayerInfo>,
    mut commands: Commands,
) {
    let Ok(question_eid) = question_q.single() else {
        return;
    };

    let ai_usernames: Vec<Username> = player_info
        .named_players
        .iter()
        .filter(|p| matches!(p.control, PlayerControl::AI(_)))
        .map(|p| p.username.clone())
        .collect();

    if ai_usernames.is_empty() {
        return;
    }

    commands.entity(question_eid).insert(AIState {
        guess_attempts: ai_usernames.iter().map(|u| (u.clone(), vec![])).collect(),
        bet_attempts: ai_usernames.iter().map(|u| (u.clone(), vec![])).collect(),
    });
}

pub fn drive_ai_guesses(
    question_q: Query<(&Question, &QuestionActive)>,
    game_state_q: Query<&GameState>,
    player_info: Single<&PlayerInfo>,
    ai_clients: Option<Res<AIClients>>,
    mut tasks: ResMut<AITasksInFlight>,
    mut input_queue: ResMut<GameInputQueue>,
    mut ai_state_q: Query<&mut AIState>,
) {
    let Ok((question, active)) = question_q.single() else {
        return;
    };
    if active.guess_seconds_remaining.is_none() {
        return;
    }
    let Some(ai_clients) = ai_clients else {
        return;
    };
    let Ok(_game_state) = game_state_q.single() else {
        return;
    };

    let ai_players: Vec<(Username, AIControl)> = player_info
        .named_players
        .iter()
        .filter_map(|p| match &p.control {
            PlayerControl::AI(ai) => Some((p.username.clone(), ai.clone())),
            _ => None,
        })
        .filter(|(username, _)| !question.guesses.contains_key(username))
        .collect();

    let mut completed: Vec<(Username, anyhow::Result<AIGuess>)> = vec![];
    tasks.guess_tasks.retain(
        |username, task| match future::block_on(future::poll_once(task)) {
            Some(result) => {
                completed.push((username.clone(), result));
                false
            }
            None => true,
        },
    );

    for (username, result) in completed {
        match result {
            Ok(guess) => {
                debug!("[ServerAI] {} guess → {}", username, guess.guess);
                trace!(
                    "[ServerAI] {} guess reasoning: {}",
                    username, guess.reasoning
                );
                if guess.guess > 0 {
                    input_queue.queue.push(GameInput {
                        username: username.clone(),
                        input: ClientInput::SubmitGuess { guess: guess.guess },
                    });
                } else {
                    warn!(
                        "[ServerAI] {} submitted non-positive guess: {}. Reasoning: {}",
                        username, guess.guess, guess.reasoning
                    );
                    if let Ok(mut ai_state) = ai_state_q.single_mut() {
                        if let Some(attempts) = ai_state.guess_attempts.get_mut(&username) {
                            attempts.push(AIGuessAttempt {
                                guess,
                                invalid_reason: Some(AIInvalidGuessReason::NotPositive),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("{} AI guess request failed: {}", username, e);
                warn!("{}", error_msg);
                if let Ok(mut ai_state) = ai_state_q.single_mut() {
                    if let Some(attempts) = ai_state.guess_attempts.get_mut(&username) {
                        attempts.push(AIGuessAttempt {
                            guess: AIGuess {
                                guess: 0,
                                reasoning: String::new(),
                            },
                            invalid_reason: Some(AIInvalidGuessReason::Error(error_msg)),
                        });
                    }
                }
            }
        }
    }

    let ai_state_opt = ai_state_q.single().ok();

    for (username, ai_control) in ai_players {
        if tasks.guess_tasks.contains_key(&username) {
            continue;
        }

        let attempts = ai_state_opt
            .as_ref()
            .and_then(|s| s.guess_attempts.get(&username))
            .map(|a| a.len())
            .unwrap_or(0);

        if attempts >= MAX_AI_ATTEMPTS {
            continue;
        }

        let invalid_guesses: Vec<AIInvalidGuess> = ai_state_opt
            .as_ref()
            .and_then(|s| s.guess_attempts.get(&username))
            .map(|attempts| {
                attempts
                    .iter()
                    .filter_map(|a| {
                        a.invalid_reason.as_ref().map(|r| AIInvalidGuess {
                            guess: a.guess.clone(),
                            reason: r.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            "[ServerAI] {} guess task starting (attempt {})",
            username,
            attempts + 1
        );
        let clients = ai_clients.clone();
        let q = question.question.clone();
        let control = ai_control.clone();

        let task = bevy::tasks::IoTaskPool::get().spawn(async move {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");
            rt.block_on(async {
                match control {
                    AIControl::Fermi(fermi) => fermi.make_guess(clients, q, invalid_guesses).await,
                }
            })
        });

        tasks.guess_tasks.insert(username.clone(), task);
    }
}

pub fn drive_ai_bets(
    question_q: Query<&Question, With<QuestionActive>>,
    bets_q: Query<(&Bets, &BetsActive)>,
    game_state_q: Query<&GameState>,
    player_info: Single<&PlayerInfo>,
    ai_clients: Option<Res<AIClients>>,
    mut tasks: ResMut<AITasksInFlight>,
    mut input_queue: ResMut<GameInputQueue>,
    mut ai_state_q: Query<&mut AIState>,
) {
    let Ok((bets, bets_active)) = bets_q.single() else {
        return;
    };
    if bets_active.bets_seconds_remaining.is_none() {
        return;
    }
    let Some(ai_clients) = ai_clients else {
        return;
    };
    let Ok(game_state) = game_state_q.single() else {
        return;
    };
    let Ok(question) = question_q.single() else {
        return;
    };

    let ai_players: Vec<(Username, AIControl)> = player_info
        .named_players
        .iter()
        .filter_map(|p| match &p.control {
            PlayerControl::AI(ai) => Some((p.username.clone(), ai.clone())),
            _ => None,
        })
        .filter(|(username, _)| bets_active.bets_locked.get(username) != Some(&true))
        .collect();

    let mut completed: Vec<(Username, anyhow::Result<AIBets>)> = vec![];
    tasks.bet_tasks.retain(
        |username, task| match future::block_on(future::poll_once(task)) {
            Some(result) => {
                completed.push((username.clone(), result));
                false
            }
            None => true,
        },
    );

    let valid_guess_values: std::collections::HashSet<u32> = question
        .guesses
        .values()
        .copied()
        .chain(std::iter::once(0))
        .collect();

    for (username, result) in completed {
        match result {
            Ok(ai_bets) => {
                debug!("[ServerAI] {} bets → {:?}", username, ai_bets.bets);
                trace!(
                    "[ServerAI] {} bet reasoning: {}",
                    username, ai_bets.reasoning
                );

                let invalid_guesses: Vec<u32> = ai_bets
                    .bets
                    .iter()
                    .filter(|(_, amount)| **amount > 0)
                    .filter(|(guess_value, _)| !valid_guess_values.contains(guess_value))
                    .map(|(guess_value, _)| *guess_value)
                    .collect();

                if !invalid_guesses.is_empty() {
                    warn!(
                        "[ServerAI] {} bet on invalid guess value(s): {:?}. Valid: {:?}. Full bets: {:?}",
                        username, invalid_guesses, valid_guess_values, ai_bets.bets
                    );
                    let error_msg = format!(
                        "{} bet on invalid guess value(s): {:?}. Valid values are: {:?}",
                        username, invalid_guesses, valid_guess_values
                    );
                    if let Ok(mut ai_state) = ai_state_q.single_mut() {
                        if let Some(attempts) = ai_state.bet_attempts.get_mut(&username) {
                            attempts.push(AIBetAttempt {
                                bets: ai_bets,
                                invalid_reason: Some(AIInvalidBetsReason::InvalidGuessValue(
                                    error_msg,
                                )),
                            });
                        }
                    }
                    continue;
                }

                let total_chips: u32 = ai_bets.bets.values().sum();
                if total_chips < 2 {
                    warn!(
                        "[ServerAI] {} only used {} chips (need 2). Bets: {:?}",
                        username, total_chips, ai_bets.bets
                    );
                    if let Ok(mut ai_state) = ai_state_q.single_mut() {
                        if let Some(attempts) = ai_state.bet_attempts.get_mut(&username) {
                            attempts.push(AIBetAttempt {
                                bets: ai_bets,
                                invalid_reason: Some(AIInvalidBetsReason::DidNotUseFreeChips),
                            });
                        }
                    }
                    continue;
                }

                let mut test_bets = bets.clone();
                let mut free_chips_remaining: u32 = 2;
                for (&guess_value, &amount) in &ai_bets.bets {
                    if amount == 0 {
                        continue;
                    }
                    let num_free = amount.min(free_chips_remaining);
                    let num_paid = amount.saturating_sub(num_free);
                    free_chips_remaining = free_chips_remaining.saturating_sub(num_free);
                    test_bets = test_bets.with_added_bet(
                        guess_value,
                        Bet {
                            owner: username.clone(),
                            num_free,
                            num_paid,
                        },
                    );
                }

                if let Err(e) = test_bets.validate_bets(&game_state) {
                    warn!(
                        "[ServerAI] {} bets validation failed: {}. Bets: {:?}",
                        username, e, ai_bets.bets
                    );
                    let error_msg = format!("{} submitted invalid bets: {}", username, e);
                    if let Ok(mut ai_state) = ai_state_q.single_mut() {
                        if let Some(attempts) = ai_state.bet_attempts.get_mut(&username) {
                            attempts.push(AIBetAttempt {
                                bets: ai_bets,
                                invalid_reason: Some(AIInvalidBetsReason::ValidationFailed(
                                    error_msg,
                                )),
                            });
                        }
                    }
                    continue;
                }

                let mut free_remaining: u32 = 2;
                for (&guess_value, &amount) in &ai_bets.bets {
                    if amount == 0 {
                        continue;
                    }
                    let num_free = amount.min(free_remaining);
                    let num_paid = amount.saturating_sub(num_free);
                    free_remaining = free_remaining.saturating_sub(num_free);
                    trace!(
                        "[ServerAI] {} bet: guess={}, free={}, paid={}",
                        username, guess_value, num_free, num_paid
                    );
                    input_queue.queue.push(GameInput {
                        username: username.clone(),
                        input: ClientInput::SubmitBet {
                            guess: guess_value,
                            num_free,
                            num_paid,
                        },
                    });
                }
                trace!("[ServerAI] {} locking bets", username);
                input_queue.queue.push(GameInput {
                    username: username.clone(),
                    input: ClientInput::LockBets,
                });
            }
            Err(e) => {
                let error_msg = format!("{} AI bet request failed: {}", username, e);
                warn!("{}", error_msg);
                if let Ok(mut ai_state) = ai_state_q.single_mut() {
                    if let Some(attempts) = ai_state.bet_attempts.get_mut(&username) {
                        attempts.push(AIBetAttempt {
                            bets: AIBets {
                                bets: HashMap::new(),
                                reasoning: String::new(),
                            },
                            invalid_reason: Some(AIInvalidBetsReason::Error(error_msg)),
                        });
                    }
                }
            }
        }
    }

    let ai_state_opt = ai_state_q.single().ok();

    for (username, ai_control) in ai_players {
        if tasks.bet_tasks.contains_key(&username) {
            continue;
        }

        let attempts = ai_state_opt
            .as_ref()
            .and_then(|s| s.bet_attempts.get(&username))
            .map(|a| a.len())
            .unwrap_or(0);

        if attempts >= MAX_AI_ATTEMPTS {
            continue;
        }

        let invalid_bets: Vec<AIInvalidBets> = ai_state_opt
            .as_ref()
            .and_then(|s| s.bet_attempts.get(&username))
            .map(|attempts| {
                attempts
                    .iter()
                    .filter_map(|a| {
                        a.invalid_reason.as_ref().map(|r| AIInvalidBets {
                            bets: a.bets.clone(),
                            reason: r.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            "[ServerAI] {} bet task starting (attempt {}, score={})",
            username,
            attempts + 1,
            game_state.scores.get(&username).copied().unwrap_or(0)
        );
        let clients = ai_clients.clone();
        let my_score = game_state.scores.get(&username).copied().unwrap_or(0);
        let context = BettingContext {
            question: question.question.clone(),
            answer: question.answer,
            guesses: question.guesses.clone(),
            my_score,
        };
        let control = ai_control.clone();

        let task = bevy::tasks::IoTaskPool::get().spawn(async move {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");
            rt.block_on(async {
                match control {
                    AIControl::Fermi(fermi) => {
                        fermi.make_bets(clients, context, invalid_bets).await
                    }
                }
            })
        });

        tasks.bet_tasks.insert(username.clone(), task);
    }
}

pub fn drive_ai_vote_continue(
    round_cap_q: Query<&RoundCap>,
    player_info: Single<&PlayerInfo>,
    mut input_queue: ResMut<GameInputQueue>,
) {
    let Ok(round_cap) = round_cap_q.single() else {
        return;
    };
    if round_cap.seconds_until_auto_continue.is_none() {
        return;
    }

    for named in &player_info.named_players {
        if !matches!(named.control, PlayerControl::AI(_)) {
            continue;
        }
        if round_cap.continue_locked.get(&named.username) == Some(&true) {
            continue;
        }
        input_queue.queue.push(GameInput {
            username: named.username.clone(),
            input: ClientInput::VoteContinue,
        });
    }
}

pub fn server_ai_plugin_fn(app: &mut App) {
    app.add_systems(Startup, setup_ai_clients);
}
