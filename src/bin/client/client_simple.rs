use bevy::prelude::*;
use bits::consts::{CLIENT_ADDR, SERVER_ADDR};
use lightyear::{
    netcode::{Key, NetcodeClient},
    prelude::{
        client::{ClientPlugins, NetcodeConfig, WebTransportClientIo},
        *,
    },
};

fn get_client_id_from_name(name: &str) -> u64 {
    match name {
        "A" => 0,
        "B" => 1,
        _ => panic!("Client name must be 'A' or 'B', got: {}", name),
    }
}

fn simple_client_startup(mut commands: Commands) -> Result<()> {
    let client_name = std::env::var("CLIENT_NAME").unwrap_or_else(|_| "A".to_string());
    let client_id = get_client_id_from_name(&client_name);

    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        client_id,
        private_key: Key::default(),
        protocol_id: 0,
    };

    let client = commands
        .spawn((
            Client::default(),
            lightyear::prelude::LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::new(None),
            ReplicationReceiver::default(),
            NetcodeClient::new(auth, NetcodeConfig::default())?,
            WebTransportClientIo {
                certificate_digest:
                    "8fd2fbbcb4983dbef6e12b58bb9a0d459b460cab3564dce0fd1041916707c8a8".into(),
            },
        ))
        .id();

    commands.trigger(Connect { entity: client });

    commands.spawn(Camera2d);

    Ok(())
}

pub fn client_simple_plugin_fn(app: &mut App) {
    app.add_plugins(ClientPlugins::default());
    app.add_systems(Startup, simple_client_startup);
}
