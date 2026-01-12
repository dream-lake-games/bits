use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::player::{PlayerReviewingScreen, player_reviewing_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource)]
struct ReviewingScreenEntity(Entity);

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    let initial = PlayerReviewingScreen::new(5)
        .with_total_score(42)
        .with_seconds_until_auto(30.0);

    // Insert as resource (edit buffer)
    commands.insert_resource(initial.clone());

    // Spawn as component (display)
    let entity = commands
        .spawn((Name::new("PlayerReviewingScreen"), initial))
        .id();

    commands.insert_resource(ReviewingScreenEntity(entity));
}

fn sync_on_enter(
    keyboard: Res<ButtonInput<KeyCode>>,
    resource: Res<PlayerReviewingScreen>,
    entity: Res<ReviewingScreenEntity>,
    mut screen_q: Query<&mut PlayerReviewingScreen>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(mut screen) = screen_q.get_mut(entity.0) {
            // Preserve outputs when syncing
            let cont = screen.continue_requested;
            *screen = resource.clone();
            screen.continue_requested = cont;
            info!("Synced PlayerReviewingScreen from resource");
        }
    }
}

fn log_outputs(mut screen_q: Query<&mut PlayerReviewingScreen>) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };

    if screen.continue_requested {
        info!("Continue requested!");
        screen.already_voted = true;
        screen.continue_requested = false;
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Player: Reviewing"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<PlayerReviewingScreen>::default())
        .register_type::<PlayerReviewingScreen>()
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(player_reviewing_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, (sync_on_enter, log_outputs))
        .run();
}

