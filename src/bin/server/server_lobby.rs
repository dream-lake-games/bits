use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::{Connected, LinkOf, ReplicationSender};

use crate::server_state::ServerState;

#[derive(Component)]
struct LobbyCleanup;

fn on_enter_lobby(mut commands: Commands) {
    let container = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };

    commands.spawn((
        container,
        LobbyCleanup,
        children![
            ButtonSimple::new("START")
                .with_on_release(|commands| {
                    commands.run_system_cached(start_game);
                })
                .with_disabled_system(
                    |connected_q: Query<Entity, (With<LinkOf>, With<Connected>)>| {
                        info!(
                            "mork - seeing {} connected clients",
                            connected_q.iter().count()
                        );
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

fn start_game() {
    println!("Game starting!");
}

pub fn server_lobby_plugin_fn(app: &mut App) {
    app.add_systems(OnEnter(ServerState::Lobby), on_enter_lobby);
    app.add_systems(OnExit(ServerState::Lobby), on_exit_lobby);
}
