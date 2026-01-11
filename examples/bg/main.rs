use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::bg::{BgMarker, BgSettings, bg_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

/// Tracks the previous settings to detect changes.
#[derive(Resource, Default)]
struct PreviousSettings(Option<BgSettings>);

/// Entity ID of the current background marker.
#[derive(Resource, Default)]
struct BgMarkerEntity(Option<Entity>);

fn startup(mut commands: Commands, mut bg_entity: ResMut<BgMarkerEntity>) {
    spawn_bloom_camera(&mut commands);

    let entity = commands
        .spawn((
            Name::new("BgMarker"),
            BgMarker::default(),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();

    bg_entity.0 = Some(entity);
}

fn detect_settings_change(
    mut commands: Commands,
    settings: Res<BgSettings>,
    mut previous: ResMut<PreviousSettings>,
    mut bg_entity: ResMut<BgMarkerEntity>,
) {
    let changed = match &previous.0 {
        None => true,
        Some(prev) => {
            prev.star_count != settings.star_count
                || prev.min_star_spacing != settings.min_star_spacing
                || prev.vertical_gradient_power != settings.vertical_gradient_power
                || prev.phase_timings.len() != settings.phase_timings.len()
        }
    };

    if changed {
        info!("BgSettings changed, respawning background...");

        if let Some(entity) = bg_entity.0 {
            commands.entity(entity).despawn();
        }

        let entity = commands
            .spawn((
                Name::new("BgMarker"),
                BgMarker::default(),
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();

        bg_entity.0 = Some(entity);
        previous.0 = Some(settings.clone());
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(get_window_plugin_with_title("Background Example")))
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<BgSettings>::default())
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(bg_plugin_fn)
        .init_resource::<PreviousSettings>()
        .init_resource::<BgMarkerEntity>()
        .add_systems(Startup, startup)
        .add_systems(Update, detect_settings_change)
        .run();
}
