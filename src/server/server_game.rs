use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::{input::native::ActionState, server::*, *};

use crate::server_state::ServerState;

#[derive(Component)]
struct GameCleanup;

fn on_enter_ingame(mut commands: Commands, player_info: Single<&PlayerInfo>) {
    commands.spawn((
        Name::new("GameState"),
        GameCleanup,
        Replicate::to_clients(NetworkTarget::All),
        GameState {
            round: 0,
            scores: player_info
                .named_players
                .iter()
                .map(|named| (named.username.clone(), 0))
                .collect(),
        },
    ));
}

fn on_exit_ingame(cleanup_q: Query<Entity, With<GameCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

fn server_game_invariants(
    game_state_q: Query<&GameState>,
    questions: Query<(
        &Question,
        Option<&QuestionGenerating>,
        Option<&QuestionGenerationFailed>,
        Option<&QuestionGenerated>,
        Option<&QuestionActive>,
        Option<&QuestionRetired>,
    )>,
    bets: Query<(&Bets, Option<&BetsActive>, Option<&BetsRetired>)>,
    round_cap: Query<&RoundCap>,
) {
    let _ = game_state_q.single().expect("GameState must exist");

    let mut generating_count = 0;
    let mut generation_failed_count = 0;
    let mut generated_count = 0;
    let mut active_count = 0;

    for (_, generating, failed, generated, active, retired) in questions.iter() {
        let state_count = [
            generating.is_some(),
            failed.is_some(),
            generated.is_some(),
            active.is_some(),
            retired.is_some(),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        assert!(
            state_count == 1,
            "Question must have exactly one state, found {state_count}"
        );

        if generating.is_some() {
            generating_count += 1;
        }
        if failed.is_some() {
            generation_failed_count += 1;
        }
        if generated.is_some() {
            generated_count += 1;
        }
        if active.is_some() {
            active_count += 1;
        }
    }

    assert!(
        generating_count <= 1,
        "Found {generating_count} generating questions"
    );
    assert!(
        generation_failed_count <= 1,
        "Found {generation_failed_count} generation failed questions"
    );
    assert!(
        generated_count <= 1,
        "Found {generated_count} generated questions"
    );
    assert!(active_count <= 1, "Found {active_count} active questions");

    for (_, active, retired) in bets.iter() {
        let state_count = [active.is_some(), retired.is_some()]
            .iter()
            .filter(|&&x| x)
            .count();
        assert!(
            state_count == 1,
            "Bet must have exactly one state, found {state_count}"
        );
    }

    let bets_active_count = bets
        .iter()
        .filter(|(_, active, _)| active.is_some())
        .count();
    assert!(
        bets_active_count <= 1,
        "Found {bets_active_count} bets active"
    );

    let round_cap_count = round_cap.iter().count();
    assert!(round_cap_count <= 1, "Found {round_cap_count} round caps");

    if let Some((_, bets_active, _)) = bets.iter().find(|(_, active, _)| active.is_some()) {
        let question_active = questions.iter().find_map(|(_, _, _, _, active, _)| active);
        assert!(
            question_active.is_some(),
            "BetsActive requires QuestionActive to exist"
        );
        assert!(
            question_active.unwrap().guess_seconds_remaining.is_none(),
            "BetsActive requires QuestionActive.guess_seconds_remaining to be None"
        );

        if bets_active.unwrap().bets_seconds_remaining.is_none() {
            assert!(
                round_cap_count == 1,
                "BetsActive with bets_seconds_remaining None requires RoundCap to exist"
            );
        }
    }

    if round_cap_count > 0 {
        let bets_active = bets.iter().find_map(|(_, active, _)| active);
        assert!(
            bets_active.is_some(),
            "RoundCap requires BetsActive to exist"
        );
        assert!(
            bets_active.unwrap().bets_seconds_remaining.is_none(),
            "RoundCap requires BetsActive.bets_seconds_remaining to be None"
        );
    }
}

fn update_question_names(
    mut questions: Query<
        (
            &mut Name,
            Option<&QuestionGenerating>,
            Option<&QuestionGenerationFailed>,
            Option<&QuestionGenerated>,
            Option<&QuestionActive>,
            Option<&QuestionRetired>,
        ),
        With<Question>,
    >,
) {
    for (mut name, generating, failed, generated, active, retired) in questions.iter_mut() {
        let new_name = if generating.is_some() {
            "QuestionGenerating"
        } else if failed.is_some() {
            "QuestionGenerationFailed"
        } else if generated.is_some() {
            "QuestionGenerated"
        } else if active.is_some() {
            "QuestionActive"
        } else if retired.is_some() {
            "QuestionRetired"
        } else {
            "Question"
        };

        if name.as_str() != new_name {
            *name = Name::new(new_name);
        }
    }
}

fn update_bets_names(
    mut bets: Query<(&mut Name, Option<&BetsActive>, Option<&BetsRetired>), With<Bets>>,
) {
    for (mut name, active, retired) in bets.iter_mut() {
        let new_name = if active.is_some() {
            "BetsActive"
        } else if retired.is_some() {
            "BetsRetired"
        } else {
            "Bets"
        };

        if name.as_str() != new_name {
            *name = Name::new(new_name);
        }
    }
}

fn maybe_make_active_question(
    active_question: Query<Entity, With<QuestionActive>>,
    mut generated_question: Query<(Entity, &mut Question), With<QuestionGenerated>>,
    game_state: Single<&GameState>,
    mut commands: Commands,
) {
    let Ok((generated_eid, mut question)) = generated_question.single_mut() else {
        return;
    };
    if active_question.is_empty() {
        commands.entity(generated_eid).remove::<QuestionGenerated>();
        commands.entity(generated_eid).insert(QuestionActive {
            guess_seconds_remaining: Some(30.0),
        });
        question.round = Some(game_state.round);
    }
}

fn process_guess_input(
    mut active_question_q: Query<(&mut Question, &QuestionActive)>,
    connected_remotes_q: Query<
        (&RemoteId, &ActionState<WrappedClientInput>),
        (With<ClientOf>, With<Connected>),
    >,
    player_info: Single<&PlayerInfo>,
) {
    let Ok((mut question, active)) = active_question_q.single_mut() else {
        return;
    };
    if active.guess_seconds_remaining.is_none() {
        return;
    }

    for (_remote_id, action_state) in connected_remotes_q {
        let Some(peer_id) = action_state.peer_id.clone() else {
            warn!("Client is sending messages without peer_id");
            continue;
        };

        let ClientInput::SubmitGuess { guess } = action_state.payload.clone() else {
            continue;
        };

        let Some(username) = player_info.get_username_for_peer(peer_id) else {
            warn!("Player trying to submit guess without being named");
            continue;
        };

        if question.guesses.contains_key(&username) {
            warn!("Player trying to submit guess when already submitted");
            continue;
        }

        question.guesses.insert(username, guess);
    }
}

fn finish_guessing(
    mut active_question_q: Query<(&Question, &mut QuestionActive)>,
    player_info_q: Query<&PlayerInfo>,
    time: Res<Time>,
) {
    let Ok((question, mut active)) = active_question_q.single_mut() else {
        return;
    };

    let Some(seconds_remaining) = active.guess_seconds_remaining else {
        return;
    };

    let new_seconds = seconds_remaining - time.delta_secs();
    let player_info = player_info_q.single().unwrap();
    let all_players_submitted = player_info
        .named_players
        .iter()
        .all(|named| question.guesses.contains_key(&named.username));

    if new_seconds <= 0.0 || all_players_submitted {
        active.guess_seconds_remaining = None;
    } else {
        active.guess_seconds_remaining = Some(new_seconds);
    }
}

fn maybe_make_active_bets(
    active_question_q: Query<(&Question, &QuestionActive)>,
    active_bets_q: Query<Entity, With<BetsActive>>,
    player_info_q: Query<&PlayerInfo>,
    mut commands: Commands,
) {
    if !active_bets_q.is_empty() {
        return;
    }

    let Ok((question, active)) = active_question_q.single() else {
        return;
    };

    if active.guess_seconds_remaining.is_some() {
        return;
    }

    let Some(round) = question.round else {
        warn!("Question doesn't have a round assigned");
        return;
    };

    let mut bets_map = HashMap::new();
    for &guess_value in question.guesses.values() {
        bets_map.insert(guess_value, Vec::new());
    }

    if bets_map.is_empty() {
        warn!("No guesses submitted, creating bets with no buckets");
    }

    let player_info = player_info_q.single().unwrap();
    let mut bets_locked = HashMap::new();
    for named_player in &player_info.named_players {
        bets_locked.insert(named_player.username.clone(), false);
    }

    commands.spawn((
        Name::new("Bets"),
        Replicate::to_clients(NetworkTarget::All),
        Bets {
            round,
            bets: bets_map,
        },
        BetsActive {
            bets_seconds_remaining: Some(30.0),
            bets_locked,
        },
    ));
}

fn process_bet_input(
    game_state: Single<&GameState>,
    player_info: Single<&PlayerInfo>,
    connected_remotes_q: Query<
        (&RemoteId, &ActionState<WrappedClientInput>),
        (With<ClientOf>, With<Connected>),
    >,
    active_question_q: Query<(&Question, &QuestionActive)>,
    mut active_bets_q: Query<(&mut Bets, &mut BetsActive)>,
    round_cap: Query<&RoundCap>,
) {
    if !round_cap.is_empty() {
        return;
    }

    let Ok((question, question_active)) = active_question_q.single() else {
        return;
    };
    if question_active.guess_seconds_remaining.is_some() {
        return;
    }

    let Ok((mut bets, mut bets_active)) = active_bets_q.single_mut() else {
        return;
    };
    bets.validate_bets(&game_state)
        .expect("Bets must be valid at the start of process_bet_input");

    if bets_active.bets_seconds_remaining.is_none() {
        return;
    }

    for (_remote_id, action_state) in connected_remotes_q {
        let Some(peer_id) = action_state.peer_id.clone() else {
            warn!("Client is sending messages without peer_id");
            continue;
        };

        // Handle LockBets input
        if matches!(action_state.payload, ClientInput::LockBets) {
            let Some(username) = player_info.get_username_for_peer(peer_id) else {
                warn!("Player trying to lock bets without being named");
                continue;
            };

            if bets_active.bets_locked.get(&username) == Some(&true) {
                warn!("Player trying to lock bets when already locked");
                continue;
            }

            bets_active.bets_locked.insert(username, true);
            continue;
        }

        let ClientInput::SubmitBet {
            guess,
            num_free,
            num_paid,
        } = action_state.payload.clone()
        else {
            continue;
        };

        if num_free == 0 && num_paid > 0 {
            warn!(
                "Player trying to submit invalid bet where num_free = 0 and num_paid = {num_paid}"
            );
            continue;
        }

        let Some(username) = player_info.get_username_for_peer(peer_id) else {
            warn!("Player trying to submit bet without being named");
            continue;
        };

        if bets_active.bets_locked.get(&username) == Some(&true) {
            warn!("Player trying to submit bet after locking");
            continue;
        }

        let mut valid_guesses = question.guesses.values().cloned().collect::<HashSet<_>>();
        valid_guesses.insert(0); // 0 is always valid (lowball everybody)
        if !valid_guesses.contains(&guess) {
            warn!("Player trying to submit bet on invalid guess");
            continue;
        }

        let maybe_new_bets = bets.with_added_bet(
            guess,
            Bet {
                owner: username,
                num_free,
                num_paid,
            },
        );
        if let Err(reason) = maybe_new_bets.validate_bets(&game_state) {
            warn!("Player tried to submit invalid bet based on current game state: {reason}");
            continue;
        }

        *bets = maybe_new_bets;
    }
}

fn finish_betting(
    mut active_bets_q: Query<&mut BetsActive>,
    player_info_q: Query<&PlayerInfo>,
    time: Res<Time>,
) {
    let Ok(mut active) = active_bets_q.single_mut() else {
        return;
    };

    let Some(seconds_remaining) = active.bets_seconds_remaining else {
        return;
    };

    let new_seconds = seconds_remaining - time.delta_secs();
    let player_info = player_info_q.single().unwrap();
    let all_players_locked = player_info
        .named_players
        .iter()
        .all(|named| active.bets_locked.get(&named.username) == Some(&true));

    if new_seconds <= 0.0 || all_players_locked {
        active.bets_seconds_remaining = None;
    } else {
        active.bets_seconds_remaining = Some(new_seconds);
    }
}

fn maybe_make_round_cap(
    active_bets_q: Query<(&Bets, &BetsActive)>,
    active_question_q: Query<&Question, With<QuestionActive>>,
    round_cap_q: Query<Entity, With<RoundCap>>,
    player_info_q: Query<&PlayerInfo>,
    mut commands: Commands,
    mut game_state: Single<&mut GameState>,
) {
    if !round_cap_q.is_empty() {
        return;
    }

    let Ok((bets, bets_active)) = active_bets_q.single() else {
        return;
    };
    if bets_active.bets_seconds_remaining.is_some() {
        return;
    }

    let Ok(question) = active_question_q.single() else {
        return;
    };

    let answer = question.answer;
    let valid_guesses: Vec<u32> = question
        .guesses
        .values()
        .filter(|&&g| g <= answer)
        .copied()
        .collect();
    let winning_guess = valid_guesses.into_iter().max().unwrap_or(0);
    let is_lowball = winning_guess == 0;

    let mut delta_this_round: HashMap<Username, i32> = game_state
        .scores
        .keys()
        .map(|username| (username.clone(), 0))
        .collect();

    if !is_lowball {
        for (username, &guess) in &question.guesses {
            if guess == winning_guess {
                *delta_this_round.get_mut(username).unwrap() += 1;
            }
        }
    }

    for (&guess_value, bet_list) in &bets.bets {
        let is_winning = guess_value == winning_guess;
        for bet in bet_list {
            let delta = delta_this_round.get_mut(&bet.owner).unwrap();
            if is_winning {
                let multiplier = if is_lowball { 2 } else { 1 };
                *delta += ((bet.num_free + bet.num_paid) * multiplier) as i32;
            } else {
                *delta -= bet.num_paid as i32;
            }
        }
    }

    for (username, delta) in &delta_this_round {
        let score = game_state.scores.get_mut(username).unwrap();
        *score = (*score as i32 + delta) as u32;
    }

    let player_info = player_info_q.single().unwrap();
    let mut continue_locked = HashMap::new();
    for named_player in &player_info.named_players {
        continue_locked.insert(named_player.username.clone(), false);
    }

    commands.spawn((
        Name::new("RoundCap"),
        Replicate::to_clients(NetworkTarget::All),
        RoundCap {
            seconds_until_auto_continue: Some(30.0),
            continue_locked,
            delta_this_round,
        },
    ));
}

fn process_round_cap_input(
    player_info: Single<&PlayerInfo>,
    connected_remotes_q: Query<
        (&RemoteId, &ActionState<WrappedClientInput>),
        (With<ClientOf>, With<Connected>),
    >,
    mut round_cap_q: Query<&mut RoundCap>,
) {
    let Ok(mut round_cap) = round_cap_q.single_mut() else {
        return;
    };

    if round_cap.seconds_until_auto_continue.is_none() {
        return;
    }

    for (_remote_id, action_state) in connected_remotes_q {
        let Some(peer_id) = action_state.peer_id.clone() else {
            warn!("Client is sending messages without peer_id");
            continue;
        };

        if !matches!(action_state.payload, ClientInput::VoteContinue) {
            continue;
        }

        let Some(username) = player_info.get_username_for_peer(peer_id) else {
            warn!("Player trying to vote continue without being named");
            continue;
        };

        if round_cap.continue_locked.get(&username) == Some(&true) {
            warn!("Player trying to vote continue when already voted");
            continue;
        }

        round_cap.continue_locked.insert(username, true);
    }
}

fn finish_round_cap(
    mut round_cap_q: Query<&mut RoundCap>,
    player_info_q: Query<&PlayerInfo>,
    time: Res<Time>,
) {
    let Ok(mut round_cap) = round_cap_q.single_mut() else {
        return;
    };

    let Some(seconds_remaining) = round_cap.seconds_until_auto_continue else {
        return;
    };

    let new_seconds = seconds_remaining - time.delta_secs();
    let player_info = player_info_q.single().unwrap();
    let all_players_voted = player_info
        .named_players
        .iter()
        .all(|named| round_cap.continue_locked.get(&named.username) == Some(&true));

    if new_seconds <= 0.0 || all_players_voted {
        round_cap.seconds_until_auto_continue = None;
    } else {
        round_cap.seconds_until_auto_continue = Some(new_seconds);
    }
}

fn maybe_progress_round(
    mut commands: Commands,
    mut game_state: Single<&mut GameState>,
    question_active_q: Query<Entity, With<QuestionActive>>,
    bets_active_q: Query<Entity, With<BetsActive>>,
    round_cap_q: Query<(Entity, &RoundCap)>,
) {
    let Ok((round_cap_eid, round_cap)) = round_cap_q.single() else {
        return;
    };
    if round_cap.seconds_until_auto_continue.is_some() {
        return;
    }

    let Ok(question_eid) = question_active_q.single() else {
        warn!("Expected single QuestionActive when progressing round");
        return;
    };
    let Ok(bets_eid) = bets_active_q.single() else {
        warn!("Expected single BetsActive when progressing round");
        return;
    };

    commands.entity(question_eid).remove::<QuestionActive>();
    commands.entity(question_eid).insert(QuestionRetired);

    commands.entity(bets_eid).remove::<BetsActive>();
    commands.entity(bets_eid).insert(BetsRetired);

    commands.entity(round_cap_eid).despawn();

    game_state.round += 1;
}

pub fn server_game_plugin_fn(app: &mut App) {
    app.add_systems(OnEnter(ServerState::InGame), on_enter_ingame);
    app.add_systems(
        FixedUpdate,
        (
            server_game_invariants,
            update_question_names,
            update_bets_names,
            maybe_make_active_question,
            process_guess_input,
            finish_guessing,
            maybe_make_active_bets,
            process_bet_input,
            finish_betting,
            maybe_make_round_cap,
            process_round_cap_input,
            finish_round_cap,
            maybe_progress_round,
        )
            .chain()
            .run_if(in_state(ServerState::InGame)),
    );
    app.add_systems(OnExit(ServerState::InGame), on_exit_ingame);
}
