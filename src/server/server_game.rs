use bevy::prelude::*;
use bits::prelude::*;
use lightyear::prelude::*;

use crate::{server_question::GenerateQuestion, server_state::ServerState};

fn server_game_last_invariants() {}

#[derive(Component)]
struct GameCleanup;

fn on_enter_ingame(mut commands: Commands) {
    let empty_vessel = commands
        .spawn((
            Name::new("QuestionEmpty"),
            Question::default(),
            Replicate::to_clients(NetworkTarget::All),
            GameCleanup,
        ))
        .id();
    commands.trigger(GenerateQuestion {
        entity: empty_vessel,
    });

    commands.spawn((
        FlexSimple::new().bundle(),
        GameCleanup,
        children![
            TextSimple::h1("")
                .with_text_system(
                    |question_active_q: Query<(&Question,), With<QuestionActive>>| {
                        if let Ok((question,)) = question_active_q.single() {
                            format!("Q: {}", question.question)
                        } else {
                            "Waiting for question...".to_string()
                        }
                    }
                )
                .bundle(),
        ],
    ));
}

fn on_exit_ingame(cleanup_q: Query<Entity, With<GameCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

pub fn server_game_plugin_fn(app: &mut App) {
    app.add_systems(Last, server_game_last_invariants);

    app.add_systems(OnEnter(ServerState::InGame), on_enter_ingame);
    app.add_systems(OnExit(ServerState::InGame), on_exit_ingame);
}
