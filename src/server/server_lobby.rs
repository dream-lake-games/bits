use std::collections::HashSet;

use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::{input::native::ActionState, server::ClientOf, *};

fn maintain_player_info(
    mut player_info_q: Query<&mut PlayerInfo>,
    connected_remotes_q: Query<
        (&RemoteId, &ActionState<WrappedClientInput>),
        (With<ClientOf>, With<Connected>),
    >,
) {
    let mut player_info = player_info_q.single_mut().unwrap();

    let connected_peer_id_set = connected_remotes_q
        .iter()
        .map(|r| r.0.0.clone())
        .collect::<HashSet<_>>();

    // 1. Remove bad peers (that arent' connected)
    player_info.unnamed_players = player_info
        .unnamed_players
        .iter()
        .cloned()
        .filter(|unnamed| connected_peer_id_set.contains(&unnamed.peer_id))
        .collect();
    for ix in 0..player_info.named_players.len() {
        let named_player = &mut player_info.named_players[ix];
        match &mut named_player.control {
            PlayerControl::Human(human_control) => {
                if let Some(peer_id) = human_control.peer_id {
                    if !connected_peer_id_set.contains(&peer_id) {
                        human_control.peer_id = None;
                    }
                }
            }
            _ => continue,
        }
    }

    // 2. Name new peers
    for pair in connected_remotes_q {
        let Some(peer_id) = pair.1.peer_id.clone() else {
            warn!("Client is sending messages without peer_id");
            continue;
        };
        let ClientInput::ClaimName { username } = pair.1.payload.clone() else {
            continue;
        };
        // Check if this username is already actively controlled by a human
        if player_info
            .named_players
            .iter()
            .any(|named| named.human_peer_id().is_some() && named.username == username)
        {
            warn!("Player trying to claim name that is actively in use");
            continue;
        }
        // Check if this peer already controls a human player
        if player_info
            .named_players
            .iter()
            .any(|named| named.is_human_with_peer(peer_id))
        {
            warn!("Active player trying to claim new name");
            continue;
        }
        let unnamed_ix_opt = player_info
            .unnamed_players
            .iter()
            .position(|unnamed| unnamed.peer_id == peer_id);
        let Some(unnamed_ix) = unnamed_ix_opt else {
            warn!("Player trying to claim name without firsting being seen as unnamed");
            continue;
        };
        player_info.unnamed_players.remove(unnamed_ix);

        // Find existing human player with this username (disconnected)
        let named_ix_opt = player_info
            .named_players
            .iter()
            .position(|named| named.is_human() && named.username == username);
        match named_ix_opt {
            Some(named_ix) => {
                // Reconnect to existing human player
                if let PlayerControl::Human(human) =
                    &mut player_info.named_players[named_ix].control
                {
                    human.peer_id = Some(peer_id);
                }
            }
            None => {
                // Create new human player
                player_info.named_players.push(NamedPlayer {
                    username,
                    control: PlayerControl::Human(HumanControl {
                        peer_id: Some(peer_id),
                    }),
                });
            }
        }
    }

    // 3. Create new unnamed
    for peer_id in &connected_peer_id_set {
        let is_unnamed = player_info
            .unnamed_players
            .iter()
            .any(|unnamed| unnamed.peer_id == *peer_id);
        let is_named_human = player_info
            .named_players
            .iter()
            .any(|named| named.is_human_with_peer(*peer_id));
        if !is_unnamed && !is_named_human {
            player_info.unnamed_players.push(UnnamedPlayer {
                peer_id: peer_id.clone(),
            });
        }
    }

    // 4. Check invariants
    let all_peer_ids: Vec<_> = player_info
        .unnamed_players
        .iter()
        .map(|u| u.peer_id)
        .chain(
            player_info
                .named_players
                .iter()
                .filter_map(|n| n.human_peer_id()),
        )
        .collect();
    assert!(
        all_peer_ids.len() == all_peer_ids.iter().collect::<HashSet<_>>().len(),
        "No peer id should appear twice across unnamed and named players"
    );

    let all_usernames: Vec<_> = player_info
        .named_players
        .iter()
        .map(|n| &n.username)
        .collect();
    assert!(
        all_usernames.len() == all_usernames.iter().collect::<HashSet<_>>().len(),
        "No username should appear twice in named players"
    );

    let named_with_peer_count = player_info
        .named_players
        .iter()
        .filter(|n| n.human_peer_id().is_some())
        .count();
    assert!(
        connected_peer_id_set.len() == player_info.unnamed_players.len() + named_with_peer_count,
        "Connected peers count should equal unnamed players + named players with peer_id"
    );
}

pub fn server_lobby_plugin_fn(app: &mut App) {
    app.add_systems(FixedUpdate, maintain_player_info);
}
