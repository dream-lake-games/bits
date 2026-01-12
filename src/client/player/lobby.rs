use bevy::prelude::*;
use bits::bits_ui::text::char_to_letter_anim;
use bits::bits_ui::text::AnimButton as SpriteButton;
use bits::prelude::*;

use crate::client_simple::InputsQueue;
use crate::client_state::ClientConnectionState;

const TEXT_SPEED: f32 = 0.03;

// =============================================================================
// Name Entry Screen (Players only)
// =============================================================================

#[derive(Component)]
pub struct UnnamedCleanup;

#[derive(Resource, Default)]
pub struct PendingName {
    pub letters: String,
    pub submitted: bool,
}

fn initials_text(pending: Res<PendingName>) -> String {
    let underscores = "_".repeat(3 - pending.letters.len().min(3));
    format!("{}{}", pending.letters, underscores)
}

pub fn on_enter_unnamed(mut commands: Commands) {
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

pub fn submit_name(pending: Res<PendingName>, mut inputs: ResMut<InputsQueue>) {
    if pending.submitted && !pending.letters.is_empty() {
        inputs.push(ClientInput::ClaimName {
            username: pending.letters.clone(),
        });
    }
}

pub fn on_exit_unnamed(cleanup_q: Query<Entity, With<UnnamedCleanup>>, mut commands: Commands) {
    commands.remove_resource::<PendingName>();
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Player Lobby
// =============================================================================

#[derive(Component)]
pub struct PlayerLobbyCleanup;

fn waiting_text(time: Res<Time>, mut dots: Local<u32>, mut timer: Local<f32>) -> String {
    *timer += time.delta_secs();
    if *timer > 0.5 {
        *timer = 0.0;
        *dots = (*dots + 1) % 4;
    }
    // Use '*' for visible dots, '_' for invisible placeholders
    let visible = "*".repeat(*dots as usize);
    let hidden = "_".repeat(3 - *dots as usize);
    format!("WAITING FOR HOST{}{}", visible, hidden)
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

pub fn on_enter_player_lobby(mut commands: Commands) {
    commands.spawn((
        Name::new("WaitingText"),
        PlayerLobbyCleanup,
        AnimatedText::new("WAITING FOR HOST___", UVec2::new(640, 64), TEXT_SPEED)
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

pub fn on_exit_player_lobby(
    cleanup_q: Query<Entity, With<PlayerLobbyCleanup>>,
    mut commands: Commands,
) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

// =============================================================================
// Plugin
// =============================================================================

pub fn player_lobby_plugin_fn(app: &mut App) {
    // Name entry (players only)
    app.add_systems(OnEnter(ClientConnectionState::Unnamed), on_enter_unnamed);
    app.add_systems(
        FixedUpdate,
        submit_name
            .run_if(in_state(ClientConnectionState::Unnamed))
            .run_if(resource_exists::<PendingName>),
    );
    app.add_systems(OnExit(ClientConnectionState::Unnamed), on_exit_unnamed);
}

