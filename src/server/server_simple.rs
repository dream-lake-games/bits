use std::{path::Path, time::Duration};

use bevy::prelude::*;
use bits::consts::SERVER_ADDR;
use bits::prelude::*;
use lightyear::{
    netcode::NetcodeServer,
    prelude::{server::*, *},
};
use serde::{Deserialize, Serialize};

#[derive(Resource, Clone)]
pub struct CertHash(pub String);

#[derive(Resource, Default)]
struct Registered(bool);

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
    info!("Certificate hash: {}", hash_no_colons);

    commands.insert_resource(CertHash(hash_no_colons));

    let server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(SERVER_ADDR),
            WebTransportServerIo { certificate },
        ))
        .id();

    commands.trigger(Start { entity: server });

    commands.spawn((
        Name::new("PlayerInfo"),
        PlayerInfo::default(),
        Replicate::to_clients(NetworkTarget::All),
    ));

    commands.spawn((
        Name::new("RoomInfo"),
        RoomInfo::default(),
        Replicate::to_clients(NetworkTarget::All),
    ));

    info!("=== SERVER STARTED ===");
    Ok(())
}

fn handle_server_started(trigger: On<Add, Started>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(Name::from("ServerBase"));
}

fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    info!("New client connected");
    commands.entity(trigger.entity).insert((
        ReplicationSender::new(Duration::default(), SendUpdatesMode::SinceLastAck, false),
        Name::from("Client"),
    ));
}

fn check_host_disconnect(
    room_info_q: Query<&RoomInfo>,
    connected_remotes_q: Query<&RemoteId, (With<ClientOf>, With<Connected>)>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(room_info) = room_info_q.single() else {
        return;
    };

    let Some(host_peer_id) = room_info.host_peer_id else {
        return;
    };

    let host_still_connected = connected_remotes_q
        .iter()
        .any(|remote| remote.0 == host_peer_id);

    if !host_still_connected {
        info!("Host disconnected, shutting down server");
        exit.write(AppExit::Success);
    }
}

#[derive(Serialize, Deserialize)]
struct RegisterRequest {
    room_code: String,
    cert_hash: String,
}

fn register_with_lobby(
    args: Option<Res<crate::Args>>,
    cert_hash: Option<Res<CertHash>>,
    started_q: Query<(), With<Started>>,
    mut registered: ResMut<Registered>,
) {
    if registered.0 || started_q.is_empty() {
        return;
    }

    let Some(args) = args else {
        return;
    };

    let Some(cert_hash) = cert_hash else {
        return;
    };

    let request = RegisterRequest {
        room_code: args.room_code.clone(),
        cert_hash: cert_hash.0.clone(),
    };

    let lobby_url = args.lobby_url.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = reqwest::Client::new();
            match client
                .post(format!("{}/rooms/register", lobby_url))
                .json(&request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Successfully registered with lobby");
                    } else {
                        warn!("Failed to register with lobby: {}", response.status());
                    }
                }
                Err(e) => {
                    warn!("Failed to register with lobby: {}", e);
                }
            }
        });
    });

    registered.0 = true;
}

pub fn server_simple_plugin_fn(app: &mut App) {
    app.add_plugins(ServerPlugins::default());
    app.init_resource::<Registered>();
    app.add_systems(Startup, server_simple_startup);
    app.add_systems(FixedUpdate, (check_host_disconnect, register_with_lobby));

    app.add_observer(handle_server_started);
    app.add_observer(handle_new_client);
}
