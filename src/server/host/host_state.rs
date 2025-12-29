/// Host state should be _entirely_ dependent on ServerState and replicated components.
/// These states exist as a centralized place to react to the server state and to provide
/// convenient state hooks to clue in to.
use bevy::prelude::*;
use bits::prelude::*;

use crate::server_state::ServerState;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostState {
    Lobby,
    WaitingForQuestion,
    Guessing,
    Betting,
    Reviewing,
}

fn update_host_state(
    server_state: Res<State<ServerState>>,
    mut host_state: ResMut<NextState<HostState>>,
    question_active: Query<Entity, With<QuestionActive>>,
    bets_active: Query<Entity, With<BetsActive>>,
    round_cap: Query<Entity, With<RoundCap>>,
) {
    if matches!(server_state.get(), ServerState::Lobby) {
        host_state.set(HostState::Lobby);
        return;
    }

    let has_round_cap = !round_cap.is_empty();
    let has_bets_active = !bets_active.is_empty();
    let has_question_active = !question_active.is_empty();

    let new_state = if has_round_cap {
        HostState::Reviewing
    } else if has_bets_active && has_question_active {
        HostState::Betting
    } else if has_question_active {
        HostState::Guessing
    } else {
        HostState::WaitingForQuestion
    };

    host_state.set(new_state);
}

pub fn host_state_plugin_fn(app: &mut App) {
    app.insert_state(HostState::Lobby);

    app.add_systems(FixedUpdate, update_host_state);
}
