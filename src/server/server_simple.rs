use std::{collections::HashSet, path::Path, time::Duration};

use bevy::prelude::*;
use bits::consts::SERVER_ADDR;
use bits::prelude::*;
use lightyear::{
    netcode::NetcodeServer,
    prelude::{input::native::ActionState, server::*, *},
};

fn server_simple_startup(mut commands: Commands) -> Result<()> {
    let cert_path = "certs/local_cert.pem";
    let key_path = "certs/local_key.pem";

    let rt = tokio::runtime::Runtime::new()?;

    let certificate = if Path::new(cert_path).exists() && Path::new(key_path).exists() {
        rt.block_on(async { Identity::load_pemfiles(cert_path, key_path).await })?
    } else {
        let cert = Identity::self_signed(["localhost", "127.0.0.1", "::1"])?;

        std::fs::create_dir_all("certs")?;

        rt.block_on(async {
            cert.certificate_chain().as_slice()[0]
                .store_pemfile(cert_path)
                .await?;
            cert.private_key().store_secret_pemfile(key_path).await?;
            Ok::<_, std::io::Error>(())
        })?;

        cert
    };

    let hash = certificate.certificate_chain().as_slice()[0].hash();
    let hash_no_colons = hash.to_string().replace(':', "");
    println!("Certificate hash: {}", hash_no_colons);

    let server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(SERVER_ADDR),
            WebTransportServerIo { certificate },
        ))
        .id();

    commands.trigger(Start { entity: server });
    commands.spawn(Camera2d);

    commands.spawn((
        Name::new("PlayerInfo"),
        PlayerInfo::default(),
        Replicate::to_clients(NetworkTarget::All),
    ));

    Ok(())
}

fn handle_server_started(trigger: On<Add, Started>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(Name::from("ServerBase"));
}

fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(trigger.entity).insert((
        ReplicationSender::new(Duration::default(), SendUpdatesMode::SinceLastAck, false),
        Name::from("Client"),
    ));
}

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
        if let Some(peer_id) = &player_info.named_players[ix].peer_id {
            if connected_peer_id_set.contains(peer_id) {
                continue;
            }
            player_info.named_players[ix].peer_id = None;
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
        if player_info
            .named_players
            .iter()
            .any(|named| named.peer_id.is_some() && named.username == username)
        {
            warn!("Player trying to claim name that is actively in use");
            continue;
        }
        if player_info
            .named_players
            .iter()
            .any(|named| named.peer_id == Some(peer_id))
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

        let named_ix_opt = player_info
            .named_players
            .iter()
            .position(|named| named.username == username);
        match named_ix_opt {
            Some(named_ix) => {
                player_info.named_players[named_ix].peer_id = Some(peer_id);
            }
            None => {
                player_info.named_players.push(NamedPlayer {
                    username,
                    peer_id: Some(peer_id),
                });
            }
        }
    }

    // 3. Create new unnamed
    for peer_id in &connected_peer_id_set {
        if player_info
            .unnamed_players
            .iter()
            .all(|unnamed| unnamed.peer_id != peer_id.clone())
            && player_info
                .named_players
                .iter()
                .all(|named| named.peer_id != Some(peer_id.clone()))
        {
            player_info.unnamed_players.push(UnnamedPlayer {
                peer_id: peer_id.clone(),
            });
        }
    }

    // 4. Check invariants
    let all_peer_ids: Vec<_> = player_info
        .unnamed_players
        .iter()
        .map(|u| &u.peer_id)
        .chain(
            player_info
                .named_players
                .iter()
                .filter_map(|n| n.peer_id.as_ref()),
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
        .filter(|n| n.peer_id.is_some())
        .count();
    assert!(
        connected_peer_id_set.len() == player_info.unnamed_players.len() + named_with_peer_count,
        "Connected peers count should equal unnamed players + named players with peer_id"
    );
}

pub fn server_simple_plugin_fn(app: &mut App) {
    app.add_plugins(ServerPlugins::default());
    app.add_systems(Startup, server_simple_startup);

    app.add_observer(handle_server_started);
    app.add_observer(handle_new_client);

    // This system reads input so we have to keep it in FixedUpdate since inputs are tick-synced
    app.add_systems(FixedUpdate, maintain_player_info);
}
