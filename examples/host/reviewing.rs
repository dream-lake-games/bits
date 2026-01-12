use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::host::{BetEntry, ReviewingScreen, ScoreEntry, reviewing_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource)]
struct ReviewingScreenEntity(Entity);

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    let initial = ReviewingScreen {
        question: "How many moons does Jupiter have".into(),
        units: Some("moons".into()),
        answer: 95,
        winning_guess: 79,
        winning_guessers: vec!["ALICE".into()],
        winning_bets: vec![
            BetEntry {
                player: "BOB".into(),
                amount: 2,
            },
            BetEntry {
                player: "CHARLIE".into(),
                amount: 1,
            },
        ],
        scores: vec![
            ScoreEntry {
                player: "ALICE".into(),
                score: 150,
                delta: 50,
            },
            ScoreEntry {
                player: "BOB".into(),
                score: 120,
                delta: 20,
            },
            ScoreEntry {
                player: "CHARLIE".into(),
                score: 80,
                delta: -10,
            },
        ],
        seconds_until_continue: Some(10.0),
        players_voted: vec!["ALICE".into()],
    };

    // Insert as resource (edit buffer)
    commands.insert_resource(initial.clone());

    // Spawn as component (display)
    let entity = commands.spawn((Name::new("ReviewingScreen"), initial)).id();

    commands.insert_resource(ReviewingScreenEntity(entity));
}

fn sync_on_enter(
    keyboard: Res<ButtonInput<KeyCode>>,
    resource: Res<ReviewingScreen>,
    entity: Res<ReviewingScreenEntity>,
    mut screen_q: Query<&mut ReviewingScreen>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(mut screen) = screen_q.get_mut(entity.0) {
            *screen = resource.clone();
            info!("Synced ReviewingScreen from resource");
        }
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Host: Reviewing"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<ReviewingScreen>::default())
        .register_type::<ReviewingScreen>()
        .register_type::<ScoreEntry>()
        .register_type::<BetEntry>()
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(reviewing_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, sync_on_enter)
        .run();
}
