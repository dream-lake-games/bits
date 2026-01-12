use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    // Large text at top (scale 2.0, letter_size = 64)
    commands.spawn((
        Name::new("LargeText"),
        AnimatedText::new("BIG TITLE", UVec2::new(700, 80), 0.05)
            .with_size(AnimatedTextSize::Large),
        Transform::from_xyz(0.0, 300.0, 0.0),
        Visibility::Inherited,
    ));

    // Medium text in upper middle (scale 1.0, letter_size = 32)
    commands.spawn((
        Name::new("MediumText"),
        AnimatedText::new("Medium sized text here", UVec2::new(700, 40), 0.03)
            .with_size(AnimatedTextSize::Medium),
        Transform::from_xyz(0.0, 180.0, 0.0),
        Visibility::Inherited,
    ));

    // Small text below medium (scale 0.5, letter_size = 16)
    commands.spawn((
        Name::new("SmallText"),
        AnimatedText::new(
            "Small text can fit a lot more characters in the same space!",
            UVec2::new(700, 24),
            0.02,
        )
        .with_size(AnimatedTextSize::Small),
        Transform::from_xyz(0.0, 100.0, 0.0),
        Visibility::Inherited,
    ));

    // Sentence list with small text at bottom
    commands.spawn((
        Name::new("SentenceList"),
        SentenceList::new(vec![
            "This is a sentence list".to_string(),
            "Each line appears separately".to_string(),
            "Great for showing multiple items".to_string(),
            "Like player names or scores".to_string(),
            "Or game instructions".to_string(),
        ])
        .with_size(AnimatedTextSize::Small)
        .with_text_speed(0.015),
        Transform::from_xyz(0.0, -100.0, 0.0),
        Visibility::Inherited,
    ));
}

fn main() {
    let mut app = App::new();

    let client_name = std::env::var("CLIENT_NAME").unwrap_or_else(|_| "Unknown".to_string());
    let window_title = format!("Client {}", client_name);

    app.add_plugins(
        DefaultPlugins
            .set(get_window_plugin_with_title(&window_title))
            .set(ImagePlugin::default_nearest()),
    )
    .add_plugins(EguiPlugin::default())
    .add_plugins(
        bevy_inspector_egui::quick::WorldInspectorPlugin::default()
            .run_if(input_toggle_active(false, KeyCode::Tab)),
    );

    app.add_plugins((bits_ui_plugin_fn,));

    app.add_systems(Startup, startup);

    app.run();
}
