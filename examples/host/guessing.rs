use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::host::{GuessingScreen, guessing_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource)]
struct GuessingScreenEntity(Entity);

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    let initial = GuessingScreen {
        question: "How many moons does Jupiter have".into(),
        units: Some("moons".into()),
        seconds_remaining: Some(30.0),
        players_waiting: vec!["ALICE".into(), "BOB".into()],
        players_submitted: vec!["CHARLIE".into()],
    };

    // Insert as resource (edit buffer)
    commands.insert_resource(initial.clone());

    // Spawn as component (display)
    let entity = commands.spawn((Name::new("GuessingScreen"), initial)).id();

    commands.insert_resource(GuessingScreenEntity(entity));
}

fn sync_on_enter(
    keyboard: Res<ButtonInput<KeyCode>>,
    resource: Res<GuessingScreen>,
    entity: Res<GuessingScreenEntity>,
    mut screen_q: Query<&mut GuessingScreen>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(mut screen) = screen_q.get_mut(entity.0) {
            *screen = resource.clone();
            info!("Synced GuessingScreen from resource");
        }
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Host: Guessing"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<GuessingScreen>::default())
        .register_type::<GuessingScreen>()
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(guessing_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, sync_on_enter)
        .run();
}
