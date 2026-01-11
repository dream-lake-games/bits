use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::bg::{BgMarker, BgSettings};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

/// Tracks the previous settings to detect changes.
#[derive(Resource, Default)]
struct PreviousSettings(Option<BgSettings>);

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);
}

fn detect_settings_change(
    mut commands: Commands,
    settings: Res<BgSettings>,
    mut previous: ResMut<PreviousSettings>,
    bg_q: Query<Entity, With<BgMarker>>,
) {
    let changed = match &previous.0 {
        None => {
            previous.0 = Some(settings.clone());
            false
        }
        Some(prev) => {
            prev.star_count != settings.star_count
                || prev.min_star_spacing != settings.min_star_spacing
                || prev.vertical_gradient_power != settings.vertical_gradient_power
                || prev.phase_timings.len() != settings.phase_timings.len()
        }
    };

    if changed {
        info!("BgSettings changed, respawning background...");

        if let Ok(entity) = bg_q.single() {
            commands.entity(entity).despawn();
        }

        commands.spawn((
            Name::new("BgMarker"),
            BgMarker::default(),
            Transform::default(),
            Visibility::Inherited,
        ));

        previous.0 = Some(settings.clone());
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Background Example"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<BgSettings>::default())
        .add_plugins(bits_ui_plugin_fn)
        .init_resource::<PreviousSettings>()
        .add_systems(Startup, startup)
        .add_systems(Update, detect_settings_change)
        .run();
}
