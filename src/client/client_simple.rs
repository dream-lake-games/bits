use std::collections::VecDeque;

use bevy::prelude::*;
use bits::prelude::*;
use lightyear::{
    input::client::InputSystems,
    netcode::{Key, NetcodeClient},
    prelude::{
        client::{ClientPlugins, NetcodeConfig, WebTransportClientIo},
        input::native::{ActionState, InputMarker},
        *,
    },
};
use rand::random;

fn simple_client_startup(mut commands: Commands) -> Result<()> {
    let client_id = random::<u64>();
    let peer_id = PeerId::Netcode(client_id);

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
            LocalId(peer_id),
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

    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    Ok(())
}

fn handle_client_connected(trigger: On<Add, Connected>, mut commands: Commands) {
    info!("Client connected, spawning input entity");
    commands.entity(trigger.entity).insert((
        ActionState::<WrappedClientInput>::default(),
        InputMarker::<WrappedClientInput>::default(),
        Name::new("LocalPlayerInput"),
    ));
}

const MAX_QUEUED_INPUTS: usize = 8;

#[derive(Debug, Clone, Resource, Default)]
pub struct InputsQueue {
    pub queue: VecDeque<ClientInput>,
}

impl InputsQueue {
    pub fn push(&mut self, input: ClientInput) {
        if self.queue.len() >= MAX_QUEUED_INPUTS {
            warn!("Input queue full, dropping input: {:?}", input);
            return;
        }
        self.queue.push_back(input);
    }
}

fn buffer_input(
    mut query: Query<&mut ActionState<WrappedClientInput>, With<InputMarker<WrappedClientInput>>>,
    mut inputs_queue: ResMut<InputsQueue>,
    local_id: Query<(&LocalId,), With<Connected>>,
) {
    let Ok(mut action_state) = query.single_mut() else {
        warn!("No action state to write to in client");
        return;
    };
    let Ok(peer_id) = local_id.single().map(|thing| thing.0.0.clone()) else {
        warn!("No connected peer_id to use to send buffered input");
        return;
    };

    let Some(next_action) = inputs_queue.queue.pop_front() else {
        action_state.0 = WrappedClientInput {
            peer_id: Some(peer_id),
            payload: ClientInput::Noop,
        };
        return;
    };
    action_state.0 = WrappedClientInput {
        peer_id: Some(peer_id),
        payload: next_action,
    };
}

pub fn client_simple_plugin_fn(app: &mut App) {
    app.add_plugins(ClientPlugins::default());
    app.add_systems(Startup, simple_client_startup);

    app.add_observer(handle_client_connected);

    app.insert_resource(InputsQueue::default());
    app.add_systems(
        FixedPreUpdate,
        buffer_input.in_set(InputSystems::WriteClientInputs),
    );
}
