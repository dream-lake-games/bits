use bevy::prelude::*;
use bits::bits_ui::text::AnimButton as SpriteButton;
use bits::bits_ui::text::LETTER_SIZE;
use bits::prelude::*;
use bits::window::WINDOW_SIZE;
use lightyear::prelude::{Connected, LocalId};

use crate::client_simple::InputsQueue;
use crate::client_state::{ClientConnectionState, ClientRoleState};

const TEXT_SPEED: f32 = 0.03;

// Layout constants (window is WINDOW_SIZE x WINDOW_SIZE, center at 0,0)
const HALF_WINDOW: f32 = WINDOW_SIZE as f32 / 2.0;
const PADDING: f32 = 16.0;
const HALF_LETTER: f32 = LETTER_SIZE as f32 / 2.0;

const MAX_PLAYERS: usize = 6;
const PLAYER_ROW_HEIGHT: f32 = 80.0; // 64px button + 8px padding on each side
const BUTTON_HALF_SIZE: f32 = 32.0; // Buttons are 64x64

// =============================================================================
// Host Lobby
// =============================================================================

#[derive(Component)]
pub struct HostLobbyCleanup;

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

/// Calculate the y position for a player row, centered around y=0
/// - 1 player: y=0
/// - 2 players: y=+40, y=-40
/// - 3 players: y=+80, y=0, y=-80
/// - etc.
fn calculate_row_y(index: usize, total_count: usize) -> f32 {
    if total_count == 0 {
        return 0.0;
    }
    let total_height = (total_count - 1) as f32 * PLAYER_ROW_HEIGHT;
    let top_y = total_height / 2.0;
    top_y - (index as f32 * PLAYER_ROW_HEIGHT)
}

fn get_player_count(player_info_q: &Query<&PlayerInfo>) -> usize {
    let Ok(player_info) = player_info_q.single() else {
        return 0;
    };
    player_info
        .named_players
        .iter()
        .filter(|p| match &p.control {
            PlayerControl::Human(h) => h.peer_id.is_some(),
            PlayerControl::AI(_) => true,
        })
        .count()
}

fn can_start_game(player_info_q: Query<&PlayerInfo>) -> bool {
    let count = get_player_count(&player_info_q);
    count >= 2 && count <= MAX_PLAYERS
}

fn can_add_ai(player_info_q: Query<&PlayerInfo>) -> bool {
    get_player_count(&player_info_q) < MAX_PLAYERS
}

/// Marker for a player row container in the host lobby (parent entity)
#[derive(Component)]
struct PlayerRow {
    username: String,
}

fn sync_player_rows(
    mut commands: Commands,
    player_info_q: Query<&PlayerInfo>,
    existing_rows_q: Query<(Entity, &PlayerRow, Option<&Children>)>,
) {
    let Ok(player_info) = player_info_q.single() else {
        return;
    };

    // Collect current active players
    let current_players: Vec<(String, bool)> = player_info
        .named_players
        .iter()
        .filter_map(|p| {
            let is_active = match &p.control {
                PlayerControl::Human(h) => h.peer_id.is_some(),
                PlayerControl::AI(_) => true,
            };
            if is_active {
                let is_ai = matches!(p.control, PlayerControl::AI(_));
                Some((p.username.clone(), is_ai))
            } else {
                None
            }
        })
        .collect();

    // Remove rows for players that no longer exist
    for (entity, row, children) in &existing_rows_q {
        if !current_players
            .iter()
            .any(|(name, _)| name == &row.username)
        {
            // Despawn children first, then parent
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }
            commands.entity(entity).despawn();
        }
    }

    // Add rows for new players
    let total_count = current_players.len();
    for (idx, (username, is_ai)) in current_players.iter().enumerate() {
        let already_exists = existing_rows_q
            .iter()
            .any(|(_, row, _)| &row.username == username);

        if already_exists {
            continue;
        }

        // Calculate correct y position at spawn time (avoids 1-frame pop-in at y=0)
        let row_y = calculate_row_y(idx, total_count);

        // Build the label text
        let name_text = username.clone();
        let name_width = (name_text.len() as u32 * LETTER_SIZE).max(192);

        // Spawn parent container with correct position immediately
        let mut row_entity = commands.spawn((
            Name::new(format!("PlayerRow_{}", username)),
            HostLobbyCleanup,
            PlayerRow {
                username: username.clone(),
            },
            Transform::from_xyz(0.0, row_y, 0.0),
            Visibility::Inherited,
        ));

        // Spawn text label as child (centered at parent origin)
        row_entity.with_child((
            Name::new(format!("PlayerLabel_{}", username)),
            AnimatedText::new(&name_text, UVec2::new(name_width, 32), TEXT_SPEED),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Inherited,
        ));

        // If AI, spawn remove button as child (to the right of text)
        if *is_ai {
            let button_x = (name_width as f32 / 2.0) + BUTTON_HALF_SIZE;
            let username_for_remove = username.clone();

            row_entity.with_child((
                Name::new(format!("RemoveButton_{}", username)),
                SpriteButton::new(LetterAnim::Back).with_on_release(
                    move |mut inputs: ResMut<InputsQueue>| {
                        inputs.push(ClientInput::RemoveAI {
                            username: username_for_remove.clone(),
                        });
                    },
                ),
                Transform::from_xyz(button_x, 0.0, 0.0),
                Visibility::Inherited,
            ));
        }
    }
}

fn update_player_row_positions(
    player_info_q: Query<&PlayerInfo>,
    mut rows_q: Query<(&PlayerRow, &mut Transform)>,
) {
    let Ok(player_info) = player_info_q.single() else {
        return;
    };

    // Get ordered list of active player usernames
    let active_players: Vec<&String> = player_info
        .named_players
        .iter()
        .filter(|p| match &p.control {
            PlayerControl::Human(h) => h.peer_id.is_some(),
            PlayerControl::AI(_) => true,
        })
        .map(|p| &p.username)
        .collect();

    // Update each row's position based on its index (dynamically centered)
    let total_count = active_players.len();
    for (row, mut transform) in &mut rows_q {
        if let Some(idx) = active_players
            .iter()
            .position(|name| *name == &row.username)
        {
            let row_y = calculate_row_y(idx, total_count);
            transform.translation.x = 0.0;
            transform.translation.y = row_y;
        }
    }
}

pub fn on_enter_host_lobby(mut commands: Commands) {
    // Layout constants
    let top_y = HALF_WINDOW - PADDING - HALF_LETTER;
    // Bottom: 16px padding + half button size so button is 16px from edge
    let bottom_y = -HALF_WINDOW + PADDING + BUTTON_HALF_SIZE;
    // Left half center and right half center
    let left_center_x = -HALF_WINDOW / 2.0;
    let right_center_x = HALF_WINDOW / 2.0;

    // Title at top
    commands.spawn((
        Name::new("HostTitle"),
        HostLobbyCleanup,
        AnimatedText::new("HOST LOBBY", UVec2::new(320, 32), TEXT_SPEED),
        Transform::from_xyz(0.0, top_y, 0.0),
        Visibility::Inherited,
    ));

    // Player rows are spawned dynamically by sync_player_rows system

    // Controls at bottom - two groups centered in left/right halves
    // Left half: ADD AI (label + button centered together)
    // "ADD" text (96px) + button (64px) = 160px total, so offset by 80px each side of center
    commands.spawn((
        Name::new("AddAILabel"),
        HostLobbyCleanup,
        AnimatedText::new("ADD AI", UVec2::new(192, 32), TEXT_SPEED),
        Transform::from_xyz(left_center_x - 64.0, bottom_y, 0.0),
        Visibility::Inherited,
    ));
    commands.spawn((
        Name::new("AddAIButton"),
        HostLobbyCleanup,
        SpriteButton::new(LetterAnim::Plus)
            .with_on_release(|mut inputs: ResMut<InputsQueue>| {
                inputs.push(ClientInput::AddAI);
            })
            .with_disabled_system(|q: Query<&PlayerInfo>| !can_add_ai(q)),
        Transform::from_xyz(left_center_x + 80.0, bottom_y, 0.0),
        Visibility::Inherited,
    ));

    // Right half: START (label + button centered together)
    commands.spawn((
        Name::new("StartLabel"),
        HostLobbyCleanup,
        AnimatedText::new("START", UVec2::new(160, 32), TEXT_SPEED),
        Transform::from_xyz(right_center_x - 48.0, bottom_y, 0.0),
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
        Transform::from_xyz(right_center_x + 80.0, bottom_y, 0.0),
        Visibility::Inherited,
    ));
}

pub fn on_exit_host_lobby(
    cleanup_q: Query<Entity, With<HostLobbyCleanup>>,
    mut commands: Commands,
) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Plugin
// =============================================================================

pub fn host_lobby_plugin_fn(app: &mut App) {
    // Host request sending
    app.add_systems(FixedUpdate, send_host_request);

    // Host lobby player row management (stop when game starts)
    app.add_systems(
        FixedUpdate,
        (sync_player_rows, update_player_row_positions)
            .run_if(in_state(ClientRoleState::Host))
            .run_if(in_state(ClientConnectionState::Connected))
            .run_if(|game_state_q: Query<&GameState>| game_state_q.is_empty()),
    );
}

