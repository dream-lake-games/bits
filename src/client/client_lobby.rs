use bevy::prelude::*;
use bits::bits_ui::text::Button as SpriteButton;
use bits::bits_ui::text::char_to_letter_anim;
use bits::prelude::*;
use lightyear::prelude::{Connected, LocalId};

use crate::client_simple::InputsQueue;
use crate::client_state::{ClientConnectionState, ClientRoleState};

const TEXT_SIZE: UVec2 = UVec2::new(512, 64);
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
        AnimatedText::new("HOST", UVec2::new(128, 32), TEXT_SPEED),
        Transform::from_xyz(-50.0, 50.0, 0.0),
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
        Transform::from_xyz(50.0, 50.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("PlayerLabel"),
        SelectingCleanup,
        AnimatedText::new("PLAYER", UVec2::new(192, 32), TEXT_SPEED),
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
        Transform::from_xyz(50.0, -50.0, 0.0),
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
    format!("CONNECTING{}", ".".repeat(*dots as usize))
}

fn on_enter_connecting(mut commands: Commands) {
    commands.spawn((
        Name::new("ConnectingText"),
        ConnectingCleanup,
        AnimatedText::new("CONNECTING", TEXT_SIZE, TEXT_SPEED).with_text_system(connecting_text),
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
// Name Entry Screen (Players only)
// =============================================================================

#[derive(Component)]
struct UnnamedCleanup;

#[derive(Resource, Default)]
struct PendingName {
    letters: String,
    submitted: bool,
}

fn initials_text(pending: Res<PendingName>) -> String {
    let underscores = "_".repeat(3 - pending.letters.len().min(3));
    format!("{}{}", pending.letters, underscores)
}

fn on_enter_unnamed(mut commands: Commands) {
    commands.insert_resource(PendingName::default());

    commands.spawn((
        Name::new("InitialsLabel"),
        UnnamedCleanup,
        AnimatedText::new("INITIALS", UVec2::new(256, 32), TEXT_SPEED),
        Transform::from_xyz(0.0, 200.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("InitialsDisplay"),
        UnnamedCleanup,
        AnimatedText::new("___", UVec2::new(128, 64), TEXT_SPEED).with_text_system(initials_text),
        Transform::from_xyz(0.0, 140.0, 0.0),
        Visibility::Inherited,
    ));

    let row1 = ['Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P'];
    let row2 = ['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L'];
    let row3 = ['Z', 'X', 'C', 'V', 'B', 'N', 'M'];

    let spacing = 70.0;
    let y_start = 60.0;

    for (i, &letter) in row1.iter().enumerate() {
        let x = (i as f32 - (row1.len() - 1) as f32 / 2.0) * spacing;
        spawn_letter_button(&mut commands, letter, x, y_start);
    }

    for (i, &letter) in row2.iter().enumerate() {
        let x = (i as f32 - (row2.len() - 1) as f32 / 2.0) * spacing;
        spawn_letter_button(&mut commands, letter, x, y_start - spacing);
    }

    for (i, &letter) in row3.iter().enumerate() {
        let x = (i as f32 - (row3.len() - 1) as f32 / 2.0) * spacing;
        spawn_letter_button(&mut commands, letter, x, y_start - spacing * 2.0);
    }

    // Special buttons row
    let special_y = y_start - spacing * 3.0;

    // Clear button (backtick)
    commands.spawn((
        Name::new("ClearButton"),
        UnnamedCleanup,
        SpriteButton::new(LetterAnim::Clear).with_on_release(|mut pending: ResMut<PendingName>| {
            pending.letters.clear();
        }),
        Transform::from_xyz(-spacing, special_y, 0.0),
        Visibility::Inherited,
    ));

    // Backspace button
    commands.spawn((
        Name::new("BackButton"),
        UnnamedCleanup,
        SpriteButton::new(LetterAnim::Back).with_on_release(|mut pending: ResMut<PendingName>| {
            pending.letters.pop();
        }),
        Transform::from_xyz(0.0, special_y, 0.0),
        Visibility::Inherited,
    ));

    // Submit button (plus sign, since it's "add name")
    commands.spawn((
        Name::new("SubmitButton"),
        UnnamedCleanup,
        SpriteButton::new(LetterAnim::Plus)
            .with_on_release(|mut pending: ResMut<PendingName>| {
                if pending.letters.len() >= 1 {
                    pending.submitted = true;
                }
            })
            .with_disabled_system(|pending: Res<PendingName>| pending.letters.is_empty()),
        Transform::from_xyz(spacing, special_y, 0.0),
        Visibility::Inherited,
    ));
}

fn spawn_letter_button(commands: &mut Commands, letter: char, x: f32, y: f32) {
    let anim = char_to_letter_anim(letter);
    let letter_clone = letter;
    commands.spawn((
        Name::new(format!("Key{}", letter)),
        UnnamedCleanup,
        SpriteButton::new(anim)
            .with_on_release(move |mut pending: ResMut<PendingName>| {
                if pending.letters.len() < 3 {
                    pending.letters.push(letter_clone);
                }
            })
            .with_disabled_system(|pending: Res<PendingName>| pending.letters.len() >= 3),
        Transform::from_xyz(x, y, 0.0),
        Visibility::Inherited,
    ));
}

fn submit_name(pending: Res<PendingName>, mut inputs: ResMut<InputsQueue>) {
    if pending.submitted && !pending.letters.is_empty() {
        inputs.push(ClientInput::ClaimName {
            username: pending.letters.clone(),
        });
    }
}

fn on_exit_unnamed(cleanup_q: Query<Entity, With<UnnamedCleanup>>, mut commands: Commands) {
    commands.remove_resource::<PendingName>();
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Host Lobby
// =============================================================================

#[derive(Component)]
struct HostLobbyCleanup;

fn send_host_request(
    role_state: Res<State<ClientRoleState>>,
    room_info_q: Query<&RoomInfo>,
    local_id_q: Query<&LocalId, With<Connected>>,
    mut inputs: ResMut<InputsQueue>,
) {
    if *role_state.get() != ClientRoleState::Host {
        return;
    }
    let Ok(local_id) = local_id_q.single() else {
        return;
    };
    let Ok(room_info) = room_info_q.single() else {
        return;
    };
    
    match room_info.host_peer_id {
        None => {
            // No host yet - keep requesting
            inputs.push(ClientInput::RequestHost);
        }
        Some(host_id) if host_id == local_id.0 => {
            // We are the host - done
        }
        Some(other_id) => {
            panic!("Someone else ({:?}) became host before us!", other_id);
        }
    }
}

fn host_player_list_text(player_info_q: Query<&PlayerInfo>) -> String {
    let Ok(player_info) = player_info_q.single() else {
        return "PLAYERS".to_string();
    };

    let mut lines = vec!["PLAYERS".to_string()];

    for named in &player_info.named_players {
        let status = match &named.control {
            PlayerControl::Human(h) if h.peer_id.is_some() => "",
            PlayerControl::Human(_) => " X",
            PlayerControl::AI(_) => " AI",
        };
        lines.push(format!("{}{}", named.username, status));
    }

    for _ in &player_info.unnamed_players {
        lines.push("???".to_string());
    }

    if player_info.named_players.is_empty() && player_info.unnamed_players.is_empty() {
        lines.push("NONE".to_string());
    }

    lines.join(" ")
}

fn can_start_game(player_info_q: Query<&PlayerInfo>) -> bool {
    let Ok(player_info) = player_info_q.single() else {
        return false;
    };
    let active_count = player_info
        .named_players
        .iter()
        .filter(|p| match &p.control {
            PlayerControl::Human(h) => h.peer_id.is_some(),
            PlayerControl::AI(_) => true,
        })
        .count();
    active_count >= 2
}

fn on_enter_host_lobby(mut commands: Commands) {
    commands.spawn((
        Name::new("HostTitle"),
        HostLobbyCleanup,
        AnimatedText::new("HOST LOBBY", UVec2::new(320, 64), TEXT_SPEED),
        Transform::from_xyz(0.0, 200.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("PlayerList"),
        HostLobbyCleanup,
        AnimatedText::new("PLAYERS", UVec2::new(512, 128), TEXT_SPEED)
            .with_text_system(host_player_list_text),
        Transform::from_xyz(0.0, 80.0, 0.0),
        Visibility::Inherited,
    ));

    // Add AI button
    commands.spawn((
        Name::new("AddAILabel"),
        HostLobbyCleanup,
        AnimatedText::new("ADD AI", UVec2::new(192, 32), TEXT_SPEED),
        Transform::from_xyz(-100.0, -50.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("AddAIButton"),
        HostLobbyCleanup,
        SpriteButton::new(LetterAnim::Plus).with_on_release(|mut inputs: ResMut<InputsQueue>| {
            inputs.push(ClientInput::AddAI);
        }),
        Transform::from_xyz(0.0, -50.0, 0.0),
        Visibility::Inherited,
    ));

    // Start button
    commands.spawn((
        Name::new("StartLabel"),
        HostLobbyCleanup,
        AnimatedText::new("START", UVec2::new(160, 32), TEXT_SPEED),
        Transform::from_xyz(-100.0, -130.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("StartButton"),
        HostLobbyCleanup,
        SpriteButton::new(LetterAnim::CapS)
            .with_on_release(|mut inputs: ResMut<InputsQueue>| {
                inputs.push(ClientInput::StartGame);
            })
            .with_disabled_system(|q: Query<&PlayerInfo>| !can_start_game(q)),
        Transform::from_xyz(0.0, -130.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_host_lobby(cleanup_q: Query<Entity, With<HostLobbyCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Player Lobby
// =============================================================================

#[derive(Component)]
struct PlayerLobbyCleanup;

fn waiting_text(time: Res<Time>, mut dots: Local<u32>, mut timer: Local<f32>) -> String {
    *timer += time.delta_secs();
    if *timer > 0.5 {
        *timer = 0.0;
        *dots = (*dots + 1) % 4;
    }
    format!("WAITING FOR HOST{}", ".".repeat(*dots as usize))
}

fn player_list_text(player_info_q: Query<&PlayerInfo>) -> String {
    let Ok(player_info) = player_info_q.single() else {
        return "PLAYERS".to_string();
    };

    let mut lines = vec!["PLAYERS".to_string()];

    for named in &player_info.named_players {
        let status = match &named.control {
            PlayerControl::Human(h) if h.peer_id.is_some() => "",
            PlayerControl::Human(_) => " X",
            PlayerControl::AI(_) => " AI",
        };
        lines.push(format!("{}{}", named.username, status));
    }

    if player_info.named_players.is_empty() {
        lines.push("NONE".to_string());
    }

    lines.join(" ")
}

fn on_enter_player_lobby(mut commands: Commands) {
    commands.spawn((
        Name::new("WaitingText"),
        PlayerLobbyCleanup,
        AnimatedText::new("WAITING FOR HOST", UVec2::new(512, 64), TEXT_SPEED)
            .with_text_system(waiting_text),
        Transform::from_xyz(0.0, 150.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("PlayerList"),
        PlayerLobbyCleanup,
        AnimatedText::new("PLAYERS", UVec2::new(512, 128), TEXT_SPEED)
            .with_text_system(player_list_text),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Inherited,
    ));
}

fn on_exit_player_lobby(
    cleanup_q: Query<Entity, With<PlayerLobbyCleanup>>,
    mut commands: Commands,
) {
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
    app.add_systems(OnEnter(ClientConnectionState::Connecting), on_enter_connecting);
    app.add_systems(OnExit(ClientConnectionState::Connecting), on_exit_connecting);

    // Name entry (players only)
    app.add_systems(OnEnter(ClientConnectionState::Unnamed), on_enter_unnamed);
    app.add_systems(
        FixedUpdate,
        submit_name
            .run_if(in_state(ClientConnectionState::Unnamed))
            .run_if(resource_exists::<PendingName>),
    );
    app.add_systems(OnExit(ClientConnectionState::Unnamed), on_exit_unnamed);

    // Host request sending
    app.add_systems(FixedUpdate, send_host_request);

    // Lobby screen management
    app.add_systems(
        FixedUpdate,
        manage_lobby_screens.run_if(not(in_state(ClientRoleState::Selecting))),
    );
}
