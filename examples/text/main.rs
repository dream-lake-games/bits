use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Assemble"),
        Assemble::new()
            .with_assemblable::<PlayerPixels>()
            .with_lifespan(0.5),
        Transform::default(),
        Visibility::Inherited,
    ));
}

#[derive(Assemblable)]
#[file("assets/play/letters.aseprite")]
#[tag("a")]
#[exclude_prefix("_")]
struct PlayerPixels;

fn main() {
    let mut app = App::new();

    let client_name = std::env::var("CLIENT_NAME").unwrap_or_else(|_| "Unknown".to_string());
    let window_title = format!("Client {}", client_name);

    app.add_plugins(DefaultPlugins.set(get_window_plugin_with_title(&window_title)))
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        );

    app.add_plugins((bits_ui_plugin_fn,));

    app.add_systems(Startup, startup);

    app.run();
}
