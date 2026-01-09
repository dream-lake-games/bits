use std::time::Duration;

use bevy::prelude::*;
use bits::prelude::*;

use crate::{client_simple::InputsQueue, client_state::ClientConnectionState};

#[derive(Component)]
struct DisconnectedCleanup;

fn disconnected_text(
    mut timer: Local<Option<Timer>>,
    time: Res<Time>,
    mut dot_thing: Local<u32>,
) -> String {
    let timer_duration = Duration::from_secs_f32(0.33);
    if timer.is_none() {
        *timer = Some(Timer::new(timer_duration.clone(), TimerMode::Once));
    }
    let timer = timer.as_mut().unwrap();
    timer.tick(time.delta());
    if timer.is_finished() {
        *dot_thing = (*dot_thing + 1) % 3;
        *timer = Timer::new(timer_duration.clone(), TimerMode::Once);
    }
    format!("Connecting{}", ".".repeat((*dot_thing + 1) as usize))
}

fn on_enter_disconnected(mut commands: Commands) {
    commands.spawn((
        FlexSimple::new().bundle(),
        DisconnectedCleanup,
        children![
            TextSimple::p("")
                .with_text_system(disconnected_text)
                .bundle(),
        ],
    ));
}

fn on_exit_disconnected(
    cleanup_q: Query<Entity, With<DisconnectedCleanup>>,
    mut commands: Commands,
) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

#[derive(Component)]
struct UnnamedCleanup;

#[derive(Component, Debug, Clone, Reflect, Default)]
struct PendingName {
    username: String,
    submit_pressed: bool,
    claim_sent: bool,
}

fn is_submit_disabled(pending_name: Single<&PendingName>) -> bool {
    pending_name.username.len() == 0 || pending_name.submit_pressed
}

fn are_letters_disabled(pending_name: Single<&PendingName>) -> bool {
    pending_name.username.len() >= 3 || pending_name.submit_pressed
}

fn on_enter_unnamed(mut commands: Commands) {
    commands.spawn((
        Name::new("PendingName"),
        PendingName::default(),
        UnnamedCleanup,
    ));

    const KEY_SPACING: f32 = 8.0;
    const ROW_SPACING: f32 = 10.0;
    const SECTION_SPACING: f32 = 20.0;

    const KEY_WIDTH: f32 = 50.0;
    const KEY_HEIGHT: f32 = 50.0;

    const SPECIAL_KEY_WIDTH: f32 = 100.0;
    const SPECIAL_KEY_FONT_SIZE: f32 = 16.0;

    const INITIALS_DISPLAY_WIDTH: f32 = 200.0;
    const INITIALS_DISPLAY_HEIGHT: f32 = 60.0;

    const SUBMIT_BUTTON_WIDTH: f32 = 120.0;
    const SUBMIT_BUTTON_HEIGHT: f32 = 60.0;

    let key_spacing = Val::Px(KEY_SPACING);
    let row_spacing = Val::Px(ROW_SPACING);
    let section_spacing = Val::Px(SECTION_SPACING);

    let letter_button = |letter: &str| {
        let letter_string = letter.to_string();
        ButtonSimple::small(letter)
            .with_size(Val::Px(KEY_WIDTH), Val::Px(KEY_HEIGHT))
            .with_disabled_system(are_letters_disabled)
            .with_on_release(move |mut pending_name: Single<&mut PendingName>| {
                pending_name.username = format!("{}{}", pending_name.username, letter_string);
            })
            .bundle()
    };

    commands.spawn((
        FlexSimple::new().bundle(),
        UnnamedCleanup,
        children![
            // Top section: INITIALS display and SUBMIT button
            (
                FlexSimple::new()
                    .with_direction(FlexDirection::Row)
                    .with_size(Val::Auto, Val::Auto)
                    .bundle(),
                children![
                    TextSimple::p("")
                        .with_size(
                            Val::Px(INITIALS_DISPLAY_WIDTH),
                            Val::Px(INITIALS_DISPLAY_HEIGHT)
                        )
                        .with_text_system(|pending_name: Single<&PendingName>| {
                            let username = &pending_name.username;
                            let underscores = "_".repeat(3 - username.len());
                            format!("INITIALS: {}{}", username, underscores)
                        })
                        .bundle(),
                    Spacer::width(key_spacing).bundle(),
                    ButtonSimple::small("SUBMIT")
                        .with_size(Val::Px(SUBMIT_BUTTON_WIDTH), Val::Px(SUBMIT_BUTTON_HEIGHT))
                        .with_disabled_system(is_submit_disabled)
                        .with_on_release(|mut pending_name: Single<&mut PendingName>| {
                            pending_name.submit_pressed = true;
                        })
                        .bundle(),
                ],
            ),
            Spacer::height(section_spacing).bundle(),
            // Keyboard section
            (
                FlexSimple::new()
                    .with_direction(FlexDirection::Column)
                    .with_size(Val::Auto, Val::Auto)
                    .bundle(),
                children![
                    // Row 1: Q W E R T Y U I O P
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            letter_button("Q"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("W"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("E"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("R"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("T"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("Y"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("U"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("I"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("O"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("P"),
                        ],
                    ),
                    Spacer::height(row_spacing).bundle(),
                    // Row 2: A S D F G H J K L (with offset)
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            Spacer::width(Val::Px(KEY_WIDTH / 2.0)).bundle(),
                            letter_button("A"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("S"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("D"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("F"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("G"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("H"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("J"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("K"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("L"),
                        ],
                    ),
                    Spacer::height(row_spacing).bundle(),
                    // Row 3: Z X C V B N M (with larger offset)
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            Spacer::width(Val::Px(KEY_WIDTH)).bundle(),
                            letter_button("Z"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("X"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("C"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("V"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("B"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("N"),
                            Spacer::width(key_spacing).bundle(),
                            letter_button("M"),
                        ],
                    ),
                    Spacer::height(section_spacing).bundle(),
                    // Row 4: CLEAR and BACKSPACE
                    (
                        FlexSimple::new()
                            .with_direction(FlexDirection::Row)
                            .with_size(Val::Auto, Val::Auto)
                            .bundle(),
                        children![
                            ButtonSimple::small("CLEAR")
                                .with_size(Val::Px(SPECIAL_KEY_WIDTH), Val::Px(KEY_HEIGHT))
                                .with_font_size(SPECIAL_KEY_FONT_SIZE)
                                .with_on_release(|mut pending_name: Single<&mut PendingName>| {
                                    pending_name.username.clear();
                                })
                                .bundle(),
                            Spacer::width(key_spacing).bundle(),
                            ButtonSimple::small("BACKSPACE")
                                .with_size(Val::Px(SPECIAL_KEY_WIDTH), Val::Px(KEY_HEIGHT))
                                .with_font_size(SPECIAL_KEY_FONT_SIZE)
                                .with_on_release(|mut pending_name: Single<&mut PendingName>| {
                                    pending_name.username.pop();
                                })
                                .bundle(),
                        ],
                    ),
                ],
            ),
        ],
    ));
}

fn watch_submit(mut pending_name: Single<&mut PendingName>, mut inputs_queue: ResMut<InputsQueue>) {
    if pending_name.submit_pressed && !pending_name.claim_sent {
        inputs_queue.push(ClientInput::ClaimName {
            username: pending_name.username.clone(),
        });
        pending_name.claim_sent = true;
    }
}

fn on_exit_unnamed(cleanup_q: Query<Entity, With<UnnamedCleanup>>, mut commands: Commands) {
    for ent in &cleanup_q {
        commands.entity(ent).despawn();
    }
}

pub fn client_lobby_plugin_fn(app: &mut App) {
    app.add_systems(
        OnEnter(ClientConnectionState::Disconnected),
        on_enter_disconnected,
    );
    app.add_systems(
        OnExit(ClientConnectionState::Disconnected),
        on_exit_disconnected,
    );

    app.add_systems(OnEnter(ClientConnectionState::Unnamed), on_enter_unnamed);
    app.add_systems(
        FixedUpdate,
        (watch_submit,)
            .chain()
            .run_if(in_state(ClientConnectionState::Unnamed)),
    );
    app.add_systems(OnExit(ClientConnectionState::Unnamed), on_exit_unnamed);
}
