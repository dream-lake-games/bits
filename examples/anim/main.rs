use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

include!(concat!(env!("OUT_DIR"), "/anim_registry_anim.rs"));

#[derive(Anim, Default, Clone, Copy, Debug)]
#[file("assets/play/letters.aseprite")]
#[exclude_prefix("_")]
enum LetterAnim {
    #[default]
    #[next(B)]
    A,
    B,
    #[next(ANIM_DESPAWN)]
    Poof,
}

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Letter with Assemble"),
        Assemble::new()
            .with_anim::<LetterAnim>()
            .with_lifespan(20.0),
        Transform::default(),
        Visibility::Inherited,
    ));
}

fn handle_input(keys: Res<ButtonInput<KeyCode>>, mut query: Query<&mut AnimMan<LetterAnim>>) {
    if keys.just_pressed(KeyCode::Space) {
        for mut anim_man in query.iter_mut() {
            anim_man.set(LetterAnim::Poof);
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(get_window_plugin_with_title("Anim Example")))
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(register_anims)
        .add_systems(Startup, startup)
        .add_systems(Update, handle_input)
        .run();
}
