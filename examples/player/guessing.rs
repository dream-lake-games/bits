use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::player::{PlayerGuessingScreen, player_guessing_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource)]
struct GuessingScreenEntity(Entity);

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    let initial = PlayerGuessingScreen::new("How many moons does Jupiter have")
        .with_units("moons")
        .with_seconds_remaining(45.0);

    // Insert as resource (edit buffer)
    commands.insert_resource(initial.clone());

    // Spawn as component (display)
    let entity = commands
        .spawn((Name::new("PlayerGuessingScreen"), initial))
        .id();

    commands.insert_resource(GuessingScreenEntity(entity));
}

fn sync_on_enter(
    keyboard: Res<ButtonInput<KeyCode>>,
    resource: Res<PlayerGuessingScreen>,
    entity: Res<GuessingScreenEntity>,
    mut screen_q: Query<&mut PlayerGuessingScreen>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(mut screen) = screen_q.get_mut(entity.0) {
            // Preserve local state when syncing display data
            let pending = screen.pending_guess.clone();
            let submit = screen.submit_requested;
            *screen = resource.clone();
            screen.pending_guess = pending;
            screen.submit_requested = submit;
            info!("Synced PlayerGuessingScreen from resource");
        }
    }
}

fn log_outputs(mut screen_q: Query<&mut PlayerGuessingScreen>) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };
    if screen.submit_requested {
        info!(
            "Submit requested! pending_guess = {}",
            screen.pending_guess
        );
        // Simulate server accepting the guess
        if let Ok(value) = screen.pending_guess.parse::<u32>() {
            screen.already_submitted = Some(value);
        }
        screen.submit_requested = false;
        screen.pending_guess.clear();
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Player: Guessing"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<PlayerGuessingScreen>::default())
        .register_type::<PlayerGuessingScreen>()
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(player_guessing_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, (sync_on_enter, log_outputs))
        .run();
}

