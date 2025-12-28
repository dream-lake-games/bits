use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bits::prelude::*;
use bits::protocol::ProtocolPlugin;
use bits::window::get_window_plugin_with_title;

mod client_lobby;
mod client_simple;
mod client_state;

fn play(mut _commands: Commands) {
    // const SPACING: f32 = 10.0;
    // const SECTION_SPACING: f32 = 20.0;
    // const BUTTON_WIDTH: f32 = 80.0;
    // const BUTTON_HEIGHT: f32 = 50.0;
    // const GUESS_WIDTH: f32 = 200.0;
    // const GUESS_HEIGHT: f32 = 50.0;
    // const SUBMIT_WIDTH: f32 = 270.0;
    // const SUBMIT_HEIGHT: f32 = 50.0;
    // const CLEAR_FONT_SIZE: f32 = 18.0;

    // let spacing = Val::Px(SPACING);
    // let section_spacing = Val::Px(SECTION_SPACING);

    // commands.spawn((
    //     FlexSimple::new().bundle(),
    //     children![
    //         TextSimple::p("0")
    //             .with_size(Val::Px(GUESS_WIDTH), Val::Px(GUESS_HEIGHT))
    //             .with_bg_color(Color::srgb(0.1, 0.1, 0.15))
    //             .with_border(Val::Px(2.0), Color::srgb(0.3, 0.3, 0.4))
    //             .bundle(),
    //         Spacer::height(section_spacing).bundle(),
    //         (
    //             FlexSimple::new()
    //                 .with_direction(FlexDirection::Column)
    //                 .with_size(Val::Auto, Val::Auto)
    //                 .bundle(),
    //             children![
    //                 // Row: 1 2 3
    //                 (
    //                     FlexSimple::new()
    //                         .with_direction(FlexDirection::Row)
    //                         .with_size(Val::Auto, Val::Auto)
    //                         .bundle(),
    //                     children![
    //                         ButtonSimple::small("1")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("2")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("3")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                     ],
    //                 ),
    //                 Spacer::height(spacing).bundle(),
    //                 // Row: 4 5 6
    //                 (
    //                     FlexSimple::new()
    //                         .with_direction(FlexDirection::Row)
    //                         .with_size(Val::Auto, Val::Auto)
    //                         .bundle(),
    //                     children![
    //                         ButtonSimple::small("4")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("5")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("6")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                     ],
    //                 ),
    //                 Spacer::height(spacing).bundle(),
    //                 // Row: 7 8 9
    //                 (
    //                     FlexSimple::new()
    //                         .with_direction(FlexDirection::Row)
    //                         .with_size(Val::Auto, Val::Auto)
    //                         .bundle(),
    //                     children![
    //                         ButtonSimple::small("7")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("8")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("9")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                     ],
    //                 ),
    //                 Spacer::height(spacing).bundle(),
    //                 // Row: Clr 0 ⌫
    //                 (
    //                     FlexSimple::new()
    //                         .with_direction(FlexDirection::Row)
    //                         .with_size(Val::Auto, Val::Auto)
    //                         .bundle(),
    //                     children![
    //                         ButtonSimple::small("CLR")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .with_font_size(CLEAR_FONT_SIZE)
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("0")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                         Spacer::width(spacing).bundle(),
    //                         ButtonSimple::small("DEL")
    //                             .with_size(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT))
    //                             .bundle(),
    //                     ],
    //                 ),
    //             ],
    //         ),
    //         Spacer::height(section_spacing).bundle(),
    //         ButtonSimple::small("Submit")
    //             .with_size(Val::Px(SUBMIT_WIDTH), Val::Px(SUBMIT_HEIGHT))
    //             .bundle(),
    //     ],
    // ));
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

    app.add_plugins((
        ProtocolPlugin,
        bits_ui_plugin_fn,
        client_lobby::client_lobby_plugin_fn,
        client_simple::client_simple_plugin_fn,
        client_state::client_state_plugin_fn,
    ));

    app.add_systems(Startup, play);

    app.run();
}
