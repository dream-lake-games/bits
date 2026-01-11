use bevy::prelude::*;
use bits::bits_ui::text::Button as SpriteButton;
use bits::prelude::*;

use crate::client_state::{ClientConnectionState, ClientRoleState};
use crate::host::lobby::{HostLobbyCleanup, on_enter_host_lobby, on_exit_host_lobby};
use crate::player::lobby::{PlayerLobbyCleanup, on_enter_player_lobby, on_exit_player_lobby};

const TEXT_SIZE: UVec2 = UVec2::new(512, 64);
const LABEL_SIZE: UVec2 = UVec2::new(256, 32);
const TEXT_SPEED: f32 = 0.03;

// =============================================================================
// Role Selection Screen
// =============================================================================

#[derive(Component)]
struct SelectingCleanup;

fn on_enter_selecting(mut commands: Commands) {
    commands.spawn((
        Name::new("Title"),
        SelectingCleanup,
        AnimatedText::new("FERMI", UVec2::new(256, 64), TEXT_SPEED),
        Transform::from_xyz(0.0, 150.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("HostLabel"),
        SelectingCleanup,
        AnimatedText::new("HOST", LABEL_SIZE, TEXT_SPEED),
        Transform::from_xyz(-34.0, 50.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("HostButton"),
        SelectingCleanup,
        SpriteButton::new(LetterAnim::CapH).with_on_release(
            |mut role_state: ResMut<NextState<ClientRoleState>>| {
                role_state.set(ClientRoleState::Host);
            },
        ),
        Transform::from_xyz(66.0, 50.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("PlayerLabel"),
        SelectingCleanup,
        AnimatedText::new("PLAYER", LABEL_SIZE, TEXT_SPEED),
        Transform::from_xyz(-50.0, -50.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("PlayerButton"),
        SelectingCleanup,
        SpriteButton::new(LetterAnim::CapP).with_on_release(
            |mut role_state: ResMut<NextState<ClientRoleState>>| {
                role_state.set(ClientRoleState::Player);
            },
        ),
        Transform::from_xyz(82.0, -50.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_selecting(cleanup_q: Query<Entity, With<SelectingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Connecting Screen
// =============================================================================

#[derive(Component)]
struct ConnectingCleanup;

fn connecting_text(time: Res<Time>, mut dots: Local<u32>, mut timer: Local<f32>) -> String {
    *timer += time.delta_secs();
    if *timer > 0.4 {
        *timer = 0.0;
        *dots = (*dots + 1) % 4;
    }
    // Use '*' for visible dots, '_' for invisible placeholders (pre-allocated space)
    let visible = "*".repeat(*dots as usize);
    let hidden = "_".repeat(3 - *dots as usize);
    format!("CONNECTING{}{}", visible, hidden)
}

fn on_enter_connecting(mut commands: Commands) {
    commands.spawn((
        Name::new("ConnectingText"),
        ConnectingCleanup,
        AnimatedText::new("CONNECTING___", TEXT_SIZE, TEXT_SPEED).with_text_system(connecting_text),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_connecting(cleanup_q: Query<Entity, With<ConnectingCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Screen Management
// =============================================================================

#[derive(Resource, Default)]
struct LobbyScreenState {
    host_lobby_active: bool,
    player_lobby_active: bool,
}

fn is_host_lobby_active(
    role: &State<ClientRoleState>,
    conn: &State<ClientConnectionState>,
    game_state_q: &Query<&GameState>,
) -> bool {
    *role.get() == ClientRoleState::Host
        && *conn.get() == ClientConnectionState::Connected
        && game_state_q.is_empty()
}

fn is_player_lobby_active(
    role: &State<ClientRoleState>,
    conn: &State<ClientConnectionState>,
    game_state_q: &Query<&GameState>,
) -> bool {
    *role.get() == ClientRoleState::Player
        && matches!(conn.get(), ClientConnectionState::Named { .. })
        && game_state_q.is_empty()
}

fn manage_lobby_screens(
    mut commands: Commands,
    role: Res<State<ClientRoleState>>,
    conn: Res<State<ClientConnectionState>>,
    game_state_q: Query<&GameState>,
    host_cleanup_q: Query<Entity, With<HostLobbyCleanup>>,
    player_cleanup_q: Query<Entity, With<PlayerLobbyCleanup>>,
    mut state: ResMut<LobbyScreenState>,
) {
    let should_host = is_host_lobby_active(&role, &conn, &game_state_q);
    let should_player = is_player_lobby_active(&role, &conn, &game_state_q);

    if should_host && !state.host_lobby_active {
        on_enter_host_lobby(commands.reborrow());
        state.host_lobby_active = true;
    } else if !should_host && state.host_lobby_active {
        on_exit_host_lobby(host_cleanup_q, commands.reborrow());
        state.host_lobby_active = false;
    }

    if should_player && !state.player_lobby_active {
        on_enter_player_lobby(commands.reborrow());
        state.player_lobby_active = true;
    } else if !should_player && state.player_lobby_active {
        on_exit_player_lobby(player_cleanup_q, commands.reborrow());
        state.player_lobby_active = false;
    }
}

// =============================================================================
// Plugin
// =============================================================================

pub fn client_lobby_plugin_fn(app: &mut App) {
    app.init_resource::<LobbyScreenState>();

    // Role selection
    app.add_systems(OnEnter(ClientRoleState::Selecting), on_enter_selecting);
    app.add_systems(OnExit(ClientRoleState::Selecting), on_exit_selecting);

    // Connecting (for both roles)
    app.add_systems(
        OnEnter(ClientConnectionState::Connecting),
        on_enter_connecting,
    );
    app.add_systems(
        OnExit(ClientConnectionState::Connecting),
        on_exit_connecting,
    );

    // Lobby screen management
    app.add_systems(
        FixedUpdate,
        manage_lobby_screens.run_if(not(in_state(ClientRoleState::Selecting))),
    );
}
