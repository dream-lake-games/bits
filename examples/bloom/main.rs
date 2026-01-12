use bevy::input::common_conditions::input_toggle_active;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::bits_ui::text::AnimButton as SpriteButton;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    // Standard M button
    commands.spawn((
        Name::new("Button M"),
        SpriteButton::new(LetterAnim::CapM).with_on_release(|| {
            info!("M pressed!");
        }),
        Transform::from_xyz(0.0, 80.0, 0.0),
        Visibility::Inherited,
    ));

    // 10 light green orbs showing bloom intensity gradient
    // Base color: light green (0.6, 1.0, 0.4)
    // Multipliers go from 0.5 (dim, no bloom) to 8.0 (crazy bloom)
    let multipliers = [0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0];
    let base_green = (0.6, 1.0, 0.4); // Light green

    let orb_size = 24.0;
    let spacing = 50.0;
    let start_x = -((multipliers.len() as f32 - 1.0) / 2.0) * spacing;

    for (i, &mult) in multipliers.iter().enumerate() {
        let x = start_x + i as f32 * spacing;
        let color = Color::srgb(
            base_green.0 * mult,
            base_green.1 * mult,
            base_green.2 * mult,
        );

        commands.spawn((
            Name::new(format!("Orb {} (×{:.1})", i + 1, mult)),
            Sprite {
                color,
                custom_size: Some(Vec2::splat(orb_size)),
                ..default()
            },
            Transform::from_xyz(x, -20.0, 0.0),
        ));
    }

    info!("Bloom Example (Neon Style)");
    info!("─────────────────────────────────────────────────");
    info!("Controls:");
    info!("  UP/DOWN    - Adjust bloom intensity");
    info!("  SPACE      - Toggle Additive / EnergyConserving mode");
    info!("  LEFT/RIGHT - Adjust low_frequency_boost (spread)");
    info!("  TAB        - Open inspector");
    info!("─────────────────────────────────────────────────");
    info!("Current: EnergyConserving mode");
}

fn adjust_bloom(keyboard: Res<ButtonInput<KeyCode>>, mut bloom_query: Query<&mut Bloom>) {
    let Ok(mut bloom) = bloom_query.single_mut() else {
        return;
    };

    // Intensity (UP/DOWN)
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        bloom.intensity = (bloom.intensity + 0.05).min(1.0);
        info!("Bloom intensity: {:.2}", bloom.intensity);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        bloom.intensity = (bloom.intensity - 0.05).max(0.0);
        info!("Bloom intensity: {:.2}", bloom.intensity);
    }

    // Low frequency boost / spread (LEFT/RIGHT)
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        bloom.low_frequency_boost = (bloom.low_frequency_boost + 0.1).min(1.0);
        info!(
            "Low frequency boost (spread): {:.1}",
            bloom.low_frequency_boost
        );
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        bloom.low_frequency_boost = (bloom.low_frequency_boost - 0.1).max(0.0);
        info!(
            "Low frequency boost (spread): {:.1}",
            bloom.low_frequency_boost
        );
    }

    // Toggle composite mode (SPACE)
    if keyboard.just_pressed(KeyCode::Space) {
        bloom.composite_mode = match bloom.composite_mode {
            BloomCompositeMode::EnergyConserving => {
                info!("Switched to ADDITIVE mode (brighter, blown out, old-school)");
                BloomCompositeMode::Additive
            }
            BloomCompositeMode::Additive => {
                info!("Switched to ENERGY CONSERVING mode (realistic, balanced)");
                BloomCompositeMode::EnergyConserving
            }
        };
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Bloom Example"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(bits_ui_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, adjust_bloom)
        .run();
}
