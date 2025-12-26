use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::*;

use crate::server_state::ServerState;

#[derive(Component)]
struct LobbyCleanup;

fn on_enter_lobby(mut commands: Commands) {
    commands.spawn((
        FlexSimple::new().bundle(),
        LobbyCleanup,
        children![
            TextSimple::h1("")
                .with_text_system(
                    |connected_q: Query<Entity, (With<LinkOf>, With<Connected>)>| {
                        let num_connected = connected_q.iter().count();
                        format!("Connected: {} / 2", num_connected)
                    }
                )
                .bundle(),
            Spacer::height(Val::Px(20.0)).bundle(),
            ButtonSimple::new("START")
                .with_on_release(|commands| {
                    commands.run_system_cached(start_game);
                })
                .with_disabled_system(
                    |connected_q: Query<Entity, (With<LinkOf>, With<Connected>)>| {
                        connected_q.iter().count() < 2
                    }
                )
                .bundle()
        ],
    ));
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
    app.add_systems(OnExit(ServerState::Lobby), on_exit_lobby);
}
