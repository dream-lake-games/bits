use bevy::{ecs::system::BoxedSystem, prelude::*};

#[derive(Clone, Copy, Debug)]
pub struct ButtonSimpleDrawState {
    pub bg: Color,
    pub border: Color,
    pub text_color: Color,
}

impl Default for ButtonSimpleDrawState {
    fn default() -> Self {
        Self {
            bg: Color::srgb(0.15, 0.15, 0.15),
            border: Color::BLACK,
            text_color: Color::srgb(0.9, 0.9, 0.9),
        }
    }
}

#[derive(Component, Debug)]
#[require(Button, BackgroundColor, BorderColor, Name)]
pub struct ButtonSimple {
    text: String,
    font_size: f32,
    width: Val,
    height: Val,
    border_width: Val,
    pub standard_draw: ButtonSimpleDrawState,
    pub hover_draw: ButtonSimpleDrawState,
    pub press_draw: ButtonSimpleDrawState,
    pub disabled_draw: ButtonSimpleDrawState,
    pub on_press: Option<BoxedSystem<(), ()>>,
    pub on_release: Option<BoxedSystem<(), ()>>,
    pub disabled_system: Option<BoxedSystem<(), bool>>,
    is_disabled: bool,
    last_interaction: Interaction,
}

impl ButtonSimple {
    pub fn medium(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 40.0,
            width: Val::Px(150.0),
            height: Val::Px(65.0),
            border_width: Val::Px(5.0),
            standard_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.15, 0.15, 0.15),
                border: Color::BLACK,
                text_color: Color::srgb(0.9, 0.9, 0.9),
            },
            hover_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.25, 0.25, 0.25),
                border: Color::WHITE,
                text_color: Color::srgb(1.0, 1.0, 1.0),
            },
            press_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.35, 0.75, 0.35),
                border: Color::srgb(0.0, 0.5, 1.0),
                text_color: Color::srgb(1.0, 1.0, 1.0),
            },
            disabled_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.1, 0.1, 0.1),
                border: Color::srgb(0.3, 0.3, 0.3),
                text_color: Color::srgb(0.5, 0.5, 0.5),
            },
            on_press: None,
            on_release: None,
            disabled_system: None,
            is_disabled: false,
            last_interaction: Interaction::None,
        }
    }

    pub fn small(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 24.0,
            width: Val::Px(80.0),
            height: Val::Px(50.0),
            border_width: Val::Px(2.0),
            standard_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.15, 0.15, 0.15),
                border: Color::BLACK,
                text_color: Color::srgb(0.9, 0.9, 0.9),
            },
            hover_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.25, 0.25, 0.25),
                border: Color::WHITE,
                text_color: Color::srgb(1.0, 1.0, 1.0),
            },
            press_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.35, 0.75, 0.35),
                border: Color::srgb(0.0, 0.5, 1.0),
                text_color: Color::srgb(1.0, 1.0, 1.0),
            },
            disabled_draw: ButtonSimpleDrawState {
                bg: Color::srgb(0.1, 0.1, 0.1),
                border: Color::srgb(0.3, 0.3, 0.3),
                text_color: Color::srgb(0.5, 0.5, 0.5),
            },
            on_press: None,
            on_release: None,
            disabled_system: None,
            is_disabled: false,
            last_interaction: Interaction::None,
        }
    }

    pub fn with_size(mut self, width: Val, height: Val) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_border(mut self, width: Val) -> Self {
        self.border_width = width;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_on_press<M>(mut self, system: impl IntoSystem<(), (), M> + 'static) -> Self {
        self.on_press = Some(Box::new(IntoSystem::into_system(system)));
        self
    }

    pub fn with_on_release<M>(mut self, system: impl IntoSystem<(), (), M> + 'static) -> Self {
        self.on_release = Some(Box::new(IntoSystem::into_system(system)));
        self
    }

    pub fn with_disabled_system<M>(
        mut self,
        system: impl IntoSystem<(), bool, M> + 'static,
    ) -> Self {
        self.disabled_system = Some(Box::new(IntoSystem::into_system(system)));
        self
    }

    pub fn bundle(self) -> impl Bundle {
        let text = self.text.clone();
        let font_size = self.font_size;
        let width = self.width;
        let height = self.height;
        let border_width = self.border_width;
        let bg = self.standard_draw.bg;
        let border = self.standard_draw.border;
        let text_color = self.standard_draw.text_color;

        (
            Button,
            Node {
                width,
                height,
                border: UiRect::all(border_width),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            children![(
                Text::new(text),
                TextColor(text_color),
                TextFont::default().with_font_size(font_size),
            )],
            self,
        )
    }
}

fn button_disabled_check_system(world: &mut World) {
    // TODO: This exclusive system is suboptimal - runs every frame with exclusive world access.
    // Could be optimized to only run on Changed<T> for tracked components or use events.

    let mut buttons_to_check = Vec::new();
    {
        let mut query = world.query::<(Entity, &ButtonSimple)>();
        for (entity, button) in query.iter(world) {
            if button.disabled_system.is_some() {
                buttons_to_check.push(entity);
            }
        }
    }

    for entity in buttons_to_check {
        let system_opt = world
            .get_mut::<ButtonSimple>(entity)
            .expect("ButtonSimple entity should exist")
            .disabled_system
            .take();

        if let Some(mut disabled_system) = system_opt {
            disabled_system.initialize(world);
            let is_disabled = disabled_system
                .run((), world)
                .expect("ButtonSimple disabled_system should run successfully");
            disabled_system.apply_deferred(world);

            let mut button = world
                .get_mut::<ButtonSimple>(entity)
                .expect("ButtonSimple entity should exist");
            button.is_disabled = is_disabled;
            button.disabled_system = Some(disabled_system);
        }
    }

    let mut query = world.query::<&mut ButtonSimple>();
    for mut button in query.iter_mut(world) {
        if button.disabled_system.is_none() {
            button.is_disabled = false;
        }
    }
}

fn button_callback_runner_system(world: &mut World) {
    let mut press_callbacks = Vec::new();
    let mut release_callbacks = Vec::new();
    let mut buttons_to_update = Vec::new();

    {
        let mut query = world.query::<(Entity, &Interaction, &ButtonSimple)>();
        for (entity, interaction, button) in query.iter(world) {
            let current = *interaction;
            let previous = button.last_interaction;

            if !button.is_disabled {
                if current == Interaction::Pressed
                    && previous != Interaction::Pressed
                    && button.on_press.is_some()
                {
                    press_callbacks.push(entity);
                }

                if previous == Interaction::Pressed
                    && current == Interaction::Hovered
                    && button.on_release.is_some()
                {
                    release_callbacks.push(entity);
                }
            }

            if current != previous {
                buttons_to_update.push((entity, current));
            }
        }
    }

    for entity in press_callbacks {
        let system_opt = world
            .get_mut::<ButtonSimple>(entity)
            .expect("ButtonSimple entity should exist")
            .on_press
            .take();

        if let Some(mut press_system) = system_opt {
            press_system.initialize(world);
            let _ = press_system.run((), world);
            press_system.apply_deferred(world);

            let mut button = world
                .get_mut::<ButtonSimple>(entity)
                .expect("ButtonSimple entity should exist");
            button.on_press = Some(press_system);
        }
    }

    for entity in release_callbacks {
        let system_opt = world
            .get_mut::<ButtonSimple>(entity)
            .expect("ButtonSimple entity should exist")
            .on_release
            .take();

        if let Some(mut release_system) = system_opt {
            release_system.initialize(world);
            let _ = release_system.run((), world);
            release_system.apply_deferred(world);

            let mut button = world
                .get_mut::<ButtonSimple>(entity)
                .expect("ButtonSimple entity should exist");
            button.on_release = Some(release_system);
        }
    }

    for (entity, current) in buttons_to_update {
        if let Some(mut button) = world.get_mut::<ButtonSimple>(entity) {
            button.last_interaction = current;
        }
    }
}

fn button_visual_system(
    mut buttons: Query<
        (
            &Interaction,
            &ButtonSimple,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        Or<(Changed<Interaction>, Changed<ButtonSimple>)>,
    >,
    mut text_query: Query<&mut TextColor>,
) {
    for (interaction, button, mut bg_color, mut border_color, children) in &mut buttons {
        let current = *interaction;

        let draw_state = if button.is_disabled {
            &button.disabled_draw
        } else {
            match current {
                Interaction::Pressed => &button.press_draw,
                Interaction::Hovered => &button.hover_draw,
                Interaction::None => &button.standard_draw,
            }
        };

        bg_color.0 = draw_state.bg;
        border_color.set_all(draw_state.border);

        if let Some(child) = children.first() {
            if let Ok(mut text_color) = text_query.get_mut(*child) {
                text_color.0 = draw_state.text_color;
            }
        }
    }
}

pub fn button_simple_plugin_fn(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            button_disabled_check_system,
            button_visual_system,
            button_callback_runner_system,
        )
            .chain(),
    );
}
