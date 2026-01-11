use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::host::{BetEntry, BettingScreen, GuessEntry, betting_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource)]
struct BettingScreenEntity(Entity);

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    let initial = BettingScreen {
        question: "How many moons does Jupiter have".into(),
        units: Some("moons".into()),
        seconds_remaining: Some(45.0),
        players_locked: vec!["ALICE".into()],
        all_players: vec!["ALICE".into(), "BOB".into(), "CHARLIE".into()],
        guesses: vec![
            GuessEntry {
                value: 0,
                guessers: vec![],
                bets: vec![],
            },
            GuessEntry {
                value: 79,
                guessers: vec!["ALICE".into()],
                bets: vec![BetEntry {
                    player: "BOB".into(),
                    amount: 2,
                }],
            },
            GuessEntry {
                value: 95,
                guessers: vec!["BOB".into(), "CHARLIE".into()],
                bets: vec![],
            },
        ],
    };

    // Insert as resource (edit buffer)
    commands.insert_resource(initial.clone());

    // Spawn as component (display)
    let entity = commands.spawn((Name::new("BettingScreen"), initial)).id();

    commands.insert_resource(BettingScreenEntity(entity));
}

fn sync_on_enter(
    keyboard: Res<ButtonInput<KeyCode>>,
    resource: Res<BettingScreen>,
    entity: Res<BettingScreenEntity>,
    mut screen_q: Query<&mut BettingScreen>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(mut screen) = screen_q.get_mut(entity.0) {
            *screen = resource.clone();
            info!("Synced BettingScreen from resource");
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(get_window_plugin_with_title("Host: Betting")))
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<BettingScreen>::default())
        .register_type::<BettingScreen>()
        .register_type::<GuessEntry>()
        .register_type::<BetEntry>()
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(betting_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, sync_on_enter)
        .run();
}
