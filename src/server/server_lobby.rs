use std::collections::HashSet;

use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::{input::native::ActionState, server::ClientOf, *};

use crate::server_state::ServerState;

fn maintain_player_info(
    mut player_info_q: Query<&mut PlayerInfo>,
    room_info_q: Query<&RoomInfo>,
    connected_remotes_q: Query<
        (&RemoteId, &ActionState<WrappedClientInput>),
        (With<ClientOf>, With<Connected>),
    >,
) {
    let mut player_info = player_info_q.single_mut().unwrap();
    let room_info = room_info_q.single().unwrap();

    let connected_peer_id_set = connected_remotes_q
        .iter()
        .map(|r| r.0.0.clone())
        .collect::<HashSet<_>>();

    // 1. Remove bad peers (that aren't connected)
    player_info.unnamed_players = player_info
        .unnamed_players
        .iter()
        .cloned()
        .filter(|unnamed| connected_peer_id_set.contains(&unnamed.peer_id))
        .collect();
    
    // 1b. Remove host from unnamed (host is not a player!)
    if let Some(host_id) = room_info.host_peer_id {
        player_info.unnamed_players.retain(|u| u.peer_id != host_id);
    }

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
    for (_remote_id, action_state) in connected_remotes_q.iter() {
        let Some(peer_id) = action_state.peer_id.clone() else {
            continue;
        };
        let ClientInput::ClaimName { username } = action_state.payload.clone() else {
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
            warn!("Player trying to claim name without first being seen as unnamed");
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

    // 3. Create new unnamed (skip host - host is not a player!)
    for peer_id in &connected_peer_id_set {
        if room_info.host_peer_id == Some(*peer_id) {
            continue;
        }
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

    // Count: connected = host (if any) + unnamed + named_with_peer
    let host_count = if room_info.host_peer_id.is_some() { 1 } else { 0 };
    let named_with_peer_count = player_info
        .named_players
        .iter()
        .filter(|n| n.human_peer_id().is_some())
        .count();
    assert!(
        connected_peer_id_set.len() == host_count + player_info.unnamed_players.len() + named_with_peer_count,
        "Connected peers count should equal host + unnamed players + named players with peer_id"
    );
}

fn handle_host_commands(
    mut room_info_q: Query<&mut RoomInfo>,
    mut player_info_q: Query<&mut PlayerInfo>,
    mut server_state: ResMut<NextState<ServerState>>,
    connected_remotes_q: Query<
        (&RemoteId, &ActionState<WrappedClientInput>),
        (With<ClientOf>, With<Connected>),
    >,
) {
    let Ok(mut room_info) = room_info_q.single_mut() else {
        return;
    };

    for (remote_id, action_state) in connected_remotes_q.iter() {
        let Some(peer_id) = action_state.peer_id.clone() else {
            continue;
        };

        // Log non-Noop inputs
        if !matches!(action_state.payload, ClientInput::Noop) {
            info!("[HostCommands] Input from {:?}: {:?}", peer_id, action_state.payload);
        }

        match &action_state.payload {
            ClientInput::RequestHost => {
                if room_info.host_peer_id == Some(peer_id) {
                    // Already the host, ignore duplicate request
                    continue;
                }
                if room_info.host_peer_id.is_some() {
                    warn!("[HostCommands] Rejecting RequestHost - host already exists: {:?}", room_info.host_peer_id);
                    continue;
                }
                info!("[HostCommands] Assigning host to peer {:?}", peer_id);
                room_info.host_peer_id = Some(peer_id);
            }

            ClientInput::StartGame => {
                if room_info.host_peer_id != Some(remote_id.0) {
                    warn!("Non-host trying to start game");
                    continue;
                }
                let Ok(player_info) = player_info_q.single() else {
                    continue;
                };
                let active_player_count = player_info
                    .named_players
                    .iter()
                    .filter(|p| match &p.control {
                        PlayerControl::Human(h) => h.peer_id.is_some(),
                        PlayerControl::AI(_) => true,
                    })
                    .count();
                if active_player_count < 2 {
                    warn!("Host trying to start game with fewer than 2 players");
                    continue;
                }
                info!("Host starting game");
                server_state.set(ServerState::InGame);
            }

            ClientInput::AddAI => {
                if room_info.host_peer_id != Some(remote_id.0) {
                    warn!("Non-host trying to add AI");
                    continue;
                }
                let Ok(mut player_info) = player_info_q.single_mut() else {
                    continue;
                };
                let ai_count = player_info
                    .named_players
                    .iter()
                    .filter(|p| matches!(p.control, PlayerControl::AI(_)))
                    .count();
                let ai_name = format!("AI {}", ai_count + 1);
                info!("Host adding AI: {}", ai_name);
                player_info.named_players.push(NamedPlayer {
                    username: ai_name,
                    control: PlayerControl::AI(AIControl::Fermi(FermiControl {})),
                });
            }

            ClientInput::RemoveAI { username } => {
                if room_info.host_peer_id != Some(remote_id.0) {
                    warn!("Non-host trying to remove AI");
                    continue;
                }
                let Ok(mut player_info) = player_info_q.single_mut() else {
                    continue;
                };
                let before_count = player_info.named_players.len();
                player_info.named_players.retain(|p| {
                    if p.username == *username && matches!(p.control, PlayerControl::AI(_)) {
                        info!("Host removing AI: {}", username);
                        false
                    } else {
                        true
                    }
                });
                if player_info.named_players.len() == before_count {
                    warn!("Host tried to remove AI that doesn't exist: {}", username);
                }
            }

            _ => {}
        }
    }
}

pub fn server_lobby_plugin_fn(app: &mut App) {
    app.add_systems(FixedUpdate, (maintain_player_info, handle_host_commands).chain());
}
