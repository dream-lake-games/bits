/// Client state should be _entirely_ depenedent on information received from the server
/// These state exist as a centralized place to react to the server state and to provide
/// convenient state hooks to clue in to.
use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientConnectionState {
    Disconnected,
    Unnamed,
    Named { named: NamedPlayer },
}

fn update_client_connection_state(
    connection_q: Query<(&LocalId,), With<Connected>>,
    player_info_q: Query<&PlayerInfo>,
    mut client_connection_state: ResMut<NextState<ClientConnectionState>>,
) {
    let Ok(player_info) = player_info_q.single() else {
        warn!("Client is not seeing single player info component...");
        client_connection_state.set(ClientConnectionState::Disconnected);
        return;
    };
    let Ok((local_id,)) = connection_q.single() else {
        trace!("Client is not connected");
        client_connection_state.set(ClientConnectionState::Disconnected);
        return;
    };

    let peer_id = local_id.0;
    let is_unnamed = player_info
        .unnamed_players
        .iter()
        .any(|unnamed| unnamed.peer_id == peer_id);
    let named_opt = player_info
        .named_players
        .iter()
        .find(|named| named.peer_id == Some(peer_id));

    let new_state = match (is_unnamed, named_opt) {
        (true, Some(_)) => panic!("A single client peer_id should never be both unnamed and named"),
        (false, None) => {
            debug!(
                "Client is connected but neither unnamed nor named. If this does not resolve in a few RTT, something is borked."
            );
            ClientConnectionState::Disconnected
        }
        (true, None) => ClientConnectionState::Unnamed,
        (false, Some(named)) => ClientConnectionState::Named {
            named: named.clone(),
        },
    };
    client_connection_state.set(new_state);
}

pub fn client_state_plugin_fn(app: &mut App) {
    app.insert_state(ClientConnectionState::Disconnected);

    app.add_systems(Update, update_client_connection_state);
}
