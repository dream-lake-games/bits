use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::bits_ui::text::Button as SpriteButton;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource, Default)]
struct PressCounter {
    a_presses: u32,
    b_presses: u32,
    c_presses: u32,
}

#[derive(Resource, Default)]
struct DisableToggle(bool);

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    commands.insert_resource(PressCounter::default());
    commands.insert_resource(DisableToggle(false));

    // Button A - simple press counter
    commands.spawn((
        Name::new("Button A"),
        SpriteButton::new(LetterAnim::CapA).with_on_release(|mut counter: ResMut<PressCounter>| {
            counter.a_presses += 1;
            info!("A released! Total: {}", counter.a_presses);
        }),
        Transform::from_xyz(-80.0, 0.0, 0.0),
        Visibility::Inherited,
    ));

    // Button B - with both press and release
    commands.spawn((
        Name::new("Button B"),
        SpriteButton::new(LetterAnim::CapB)
            .with_on_press(|| {
                info!("B pressed!");
            })
            .with_on_release(|mut counter: ResMut<PressCounter>| {
                counter.b_presses += 1;
                info!("B released! Total: {}", counter.b_presses);
            }),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Inherited,
    ));

    // Button C - can be disabled via spacebar
    commands.spawn((
        Name::new("Button C (toggleable)"),
        SpriteButton::new(LetterAnim::CapC)
            .with_on_release(|mut counter: ResMut<PressCounter>| {
                counter.c_presses += 1;
                info!("C released! Total: {}", counter.c_presses);
            })
            .with_disabled_system(|toggle: Res<DisableToggle>| toggle.0),
        Transform::from_xyz(80.0, 0.0, 0.0),
        Visibility::Inherited,
    ));

    info!("Button Example");
    info!("- Click buttons A, B, C to increment counters");
    info!("- Press SPACE to toggle button C disabled state");
    info!("- Press TAB to open inspector");
}

fn toggle_disable(keyboard: Res<ButtonInput<KeyCode>>, mut toggle: ResMut<DisableToggle>) {
    if keyboard.just_pressed(KeyCode::Space) {
        toggle.0 = !toggle.0;
        info!(
            "Button C is now {}",
            if toggle.0 { "DISABLED" } else { "ENABLED" }
        );
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Button Example"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(bits_ui_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, toggle_disable)
        .run();
}

