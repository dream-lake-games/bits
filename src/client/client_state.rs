use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ClientRoleState {
    #[default]
    Selecting,
    Host,
    Player,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ClientConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Unnamed,
    Named { username: Username },
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientGameState {
    None,
    WaitingForQuestion,
    Guessing,
    Betting,
    Reviewing,
}

fn update_client_connection_state(
    role_state: Res<State<ClientRoleState>>,
    connection_q: Query<&LocalId, With<Connected>>,
    room_info_q: Query<&RoomInfo>,
    player_info_q: Query<&PlayerInfo>,
    current_conn_state: Res<State<ClientConnectionState>>,
    mut client_connection_state: ResMut<NextState<ClientConnectionState>>,
) {
    let Ok(local_id) = connection_q.single() else {
        client_connection_state.set(ClientConnectionState::Disconnected);
        return;
    };
    let peer_id = local_id.0;

    let Ok(room_info) = room_info_q.single() else {
        if *current_conn_state.get() != ClientConnectionState::Connecting {
            info!("[ClientState] No RoomInfo yet, transitioning to Connecting");
        }
        client_connection_state.set(ClientConnectionState::Connecting);
        return;
    };

    // Host path: never goes through Unnamed/Named - just Connected
    if *role_state.get() == ClientRoleState::Host {
        if room_info.host_peer_id == Some(peer_id) {
            if *current_conn_state.get() != ClientConnectionState::Connected {
                info!("[ClientState] Host peer_id matches! Transitioning to Connected");
            }
            client_connection_state.set(ClientConnectionState::Connected);
        } else {
            if *current_conn_state.get() != ClientConnectionState::Connecting {
                info!("[ClientState] Host waiting for host_peer_id (current: {:?}, ours: {:?})", 
                    room_info.host_peer_id, peer_id);
            }
            client_connection_state.set(ClientConnectionState::Connecting);
        }
        return;
    }

    // Player path: Unnamed → Named
    let Ok(player_info) = player_info_q.single() else {
        client_connection_state.set(ClientConnectionState::Connecting);
        return;
    };

    let is_unnamed = player_info
        .unnamed_players
        .iter()
        .any(|unnamed| unnamed.peer_id == peer_id);
    let named_opt = player_info
        .named_players
        .iter()
        .find(|named| named.is_human_with_peer(peer_id));

    let new_state = match (is_unnamed, named_opt) {
        (true, Some(_)) => panic!("A single client peer_id should never be both unnamed and named"),
        (false, None) => ClientConnectionState::Connecting,
        (true, None) => ClientConnectionState::Unnamed,
        (false, Some(named)) => ClientConnectionState::Named {
            username: named.username.clone(),
        },
    };
    client_connection_state.set(new_state);
}

fn update_client_game_state(
    client_connection_state: Res<State<ClientConnectionState>>,
    mut client_game_state: ResMut<NextState<ClientGameState>>,
    game_state_q: Query<Entity, With<GameState>>,
    question_active: Query<Entity, With<QuestionActive>>,
    bets_active: Query<Entity, With<BetsActive>>,
    round_cap: Query<Entity, With<RoundCap>>,
) {
    // Must be connected (host) or named (player) to have game state
    if matches!(
        client_connection_state.get(),
        ClientConnectionState::Disconnected
            | ClientConnectionState::Connecting
            | ClientConnectionState::Unnamed
    ) {
        client_game_state.set(ClientGameState::None);
        return;
    }

    // Game must be started (GameState entity must exist)
    if game_state_q.is_empty() {
        client_game_state.set(ClientGameState::None);
        return;
    }

    let has_round_cap = !round_cap.is_empty();
    let has_bets_active = !bets_active.is_empty();
    let has_question_active = !question_active.is_empty();

    let new_state = if has_round_cap {
        ClientGameState::Reviewing
    } else if has_bets_active && has_question_active {
        ClientGameState::Betting
    } else if has_question_active {
        ClientGameState::Guessing
    } else {
        ClientGameState::WaitingForQuestion
    };

    client_game_state.set(new_state);
}

pub fn client_state_plugin_fn(app: &mut App) {
    app.insert_state(ClientRoleState::Selecting);
    app.insert_state(ClientConnectionState::Disconnected);
    app.insert_state(ClientGameState::None);

    app.add_systems(
        FixedUpdate,
        (update_client_connection_state, update_client_game_state)
            .run_if(not(in_state(ClientRoleState::Selecting))),
    );
}
