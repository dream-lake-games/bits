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

use crate::client_state::ClientRoleState;

fn simple_client_startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);
}

fn spawn_network_client(mut commands: Commands) -> Result<()> {
    let client_id = random::<u64>();
    let peer_id = PeerId::Netcode(client_id);

    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        client_id,
        private_key: Key::default(),
        protocol_id: 0,
    };

    commands.spawn((
        Client::default(),
        lightyear::prelude::LocalAddr(CLIENT_ADDR),
        PeerAddr(SERVER_ADDR),
        LocalId(peer_id),
        Link::new(None),
        ReplicationReceiver::default(),
        NetcodeClient::new(auth, NetcodeConfig::default())?,
        WebTransportClientIo {
            certificate_digest: "2b59b22ccac6cec6720ad85f765b5724cad1f5a0e54ab5ad7b58689e0219dd09"
                .into(),
        },
    ));

    Ok(())
}

fn trigger_connect_when_ready(
    mut commands: Commands,
    role_state: Res<State<ClientRoleState>>,
    client_q: Query<Entity, (With<Client>, Without<Connected>)>,
    mut connect_triggered: Local<bool>,
) {
    if *connect_triggered {
        return;
    }
    if matches!(role_state.get(), ClientRoleState::Selecting) {
        return;
    }
    let Ok(client_entity) = client_q.single() else {
        return;
    };
    info!(
        "[Connect] Triggering connection (role: {:?})",
        role_state.get()
    );
    commands.trigger(Connect {
        entity: client_entity,
    });
    *connect_triggered = true;
}

fn handle_client_connected(trigger: On<Add, Connected>, mut commands: Commands) {
    info!("=== CLIENT CONNECTED TO SERVER ===");
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
        return;
    };
    let Ok(peer_id) = local_id.single().map(|thing| thing.0.0.clone()) else {
        return;
    };

    let Some(next_action) = inputs_queue.queue.pop_front() else {
        action_state.0 = WrappedClientInput {
            peer_id: Some(peer_id),
            payload: ClientInput::Noop,
        };
        return;
    };
    if !matches!(next_action, ClientInput::Noop) {
        info!("[BufferInput] Sending {:?}", next_action);
    }
    action_state.0 = WrappedClientInput {
        peer_id: Some(peer_id),
        payload: next_action,
    };
}

pub fn client_simple_plugin_fn(app: &mut App) {
    app.add_plugins(ClientPlugins::default());
    app.add_systems(Startup, (simple_client_startup, spawn_network_client));
    app.add_systems(Update, trigger_connect_when_ready);

    app.add_observer(handle_client_connected);

    app.insert_resource(InputsQueue::default());
    app.add_systems(
        FixedPreUpdate,
        buffer_input.in_set(InputSystems::WriteClientInputs),
    );
}
