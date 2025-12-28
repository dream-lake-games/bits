use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::*;

use crate::server_state::ServerState;

#[derive(Component)]
struct LobbyCleanup;

fn connected_count_text(connected_q: Query<Entity, (With<LinkOf>, With<Connected>)>) -> String {
    let num_connected = connected_q.iter().count();
    format!("Connected: {}", num_connected)
}

fn player_list_text(player_info_q: Query<&PlayerInfo>) -> String {
    let Ok(player_info) = player_info_q.single() else {
        return "Players:\n(no player info)".to_string();
    };

    let mut lines = vec!["Players:".to_string()];

    // Named players first
    for named in &player_info.named_players {
        let status = if named.peer_id.is_none() {
            " [DISCONNECTED]"
        } else {
            ""
        };
        lines.push(format!("  - {}{}", named.username, status));
    }

    // Then unnamed players
    for _ in &player_info.unnamed_players {
        lines.push("  - (unnamed)".to_string());
    }

    lines.join("\n")
}

fn is_start_disabled(player_info_q: Query<&PlayerInfo>) -> bool {
    let Ok(player_info) = player_info_q.single() else {
        return true;
    };

    let connected_named_count = player_info
        .named_players
        .iter()
        .filter(|named| named.peer_id.is_some())
        .count();

    connected_named_count < 2
}

fn on_enter_lobby(mut commands: Commands) {
    commands.spawn((
        FlexSimple::new().bundle(),
        LobbyCleanup,
        children![
            TextSimple::p("")
                .with_text_system(connected_count_text)
                .bundle(),
            Spacer::height(Val::Px(10.0)).bundle(),
            TextSimple::p("")
                .with_text_system(player_list_text)
                .bundle(),
            Spacer::height(Val::Px(20.0)).bundle(),
            ButtonSimple::medium("START")
                .with_on_release(start_game)
                .with_disabled_system(is_start_disabled)
                .bundle()
        ],
    ));
}

fn update_lobby(player_info_q: Query<&PlayerInfo>, mut commands: Commands) {
    let Ok(player_info) = player_info_q.single() else {
        return;
    };
    let num_fully_ready = player_info
        .named_players
        .iter()
        .filter(|named| named.peer_id.is_some())
        .count();
    if num_fully_ready >= 2 {
        commands.run_system_cached(start_game);
    }
}

fn on_exit_lobby(cleanup_q: Query<Entity, With<LobbyCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

fn start_game(mut next_server_state: ResMut<NextState<ServerState>>) {
    next_server_state.set(ServerState::InGame);
}

pub fn server_lobby_plugin_fn(app: &mut App) {
    app.add_systems(OnEnter(ServerState::Lobby), on_enter_lobby);
    app.add_systems(
        FixedUpdate,
        (update_lobby,).chain().run_if(in_state(ServerState::Lobby)),
    );
    app.add_systems(OnExit(ServerState::Lobby), on_exit_lobby);
}
