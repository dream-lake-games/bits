use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

const SENTENCES: [&str; 3] = [
    "Hello World",
    "Supercalifragilisticexpialidocious is a long word",
    "This is a very long sentence that has way too much text and should overflow the boundaries of the text box causing warnings and general chaos in the rendering system",
];

#[derive(Resource)]
struct SentenceIndex(usize);

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    commands.insert_resource(SentenceIndex(0));

    commands.spawn((
        Name::new("AnimatedText"),
        AnimatedText::new(SENTENCES[0], UVec2::new(320, 160), 0.05),
        Transform::default(),
        Visibility::Inherited,
    ));
}

fn cycle_sentence(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sentence_index: ResMut<SentenceIndex>,
    mut text_query: Query<&mut AnimatedText>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        sentence_index.0 = (sentence_index.0 + 1) % SENTENCES.len();
        for mut text in text_query.iter_mut() {
            text.text_this_frame = SENTENCES[sentence_index.0].to_string();
            text.text_last_frame = None; // Force reset
        }
    }
}

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
    app.add_systems(Update, cycle_sentence);

    app.run();
}
