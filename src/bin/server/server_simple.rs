use std::{path::Path, time::Duration};

use bevy::prelude::*;
use bits::consts::SERVER_ADDR;
use lightyear::{
    netcode::NetcodeServer,
    prelude::{server::*, *},
};

fn simple_server_startup(mut commands: Commands) -> Result<()> {
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

pub fn server_simple_plugin_fn(app: &mut App) {
    app.add_plugins(ServerPlugins::default());
    app.add_systems(Startup, simple_server_startup);

    app.add_observer(handle_server_started);
    app.add_observer(handle_new_client);
}
