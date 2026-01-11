use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::prelude::*;

/// Spawns a 2D camera with HDR bloom configured for a neon/star aesthetic.
/// Energy conserving mode, low intensity, tight glow.
pub fn spawn_bloom_camera(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("Camera"),
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Bloom {
                intensity: 0.35,
                low_frequency_boost: 0.45,
                high_pass_frequency: 0.7,
                prefilter: BloomPrefilter {
                    threshold: 1.0,
                    threshold_softness: 0.15,
                },
                composite_mode: BloomCompositeMode::EnergyConserving,
                ..default()
            },
        ))
        .id()
}

/// Convenience system for spawning the camera at startup.
pub fn spawn_bloom_camera_system(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);
}
