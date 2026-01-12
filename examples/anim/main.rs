use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    commands.spawn((
        Name::new("Letter A"),
        AnimMan::new(LetterAnim::CapA),
        Transform::from_xyz(-80.0, 0.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("Letter B"),
        AnimMan::new(LetterAnim::CapB),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Inherited,
    ));

    commands.spawn((
        Name::new("Letter C"),
        AnimMan::new(LetterAnim::CapC),
        Transform::from_xyz(80.0, 0.0, 0.0),
        Visibility::Inherited,
    ));

    info!("Anim Example - displaying letters A, B, C");
    info!("Press TAB to open inspector");
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Anim Example"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(bits_ui_plugin_fn)
        .add_systems(Startup, startup)
        .run();
}
