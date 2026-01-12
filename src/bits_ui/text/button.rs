use crate::bits_ui::anim::AnimMan;
use crate::bits_ui::colors::Palatte;
use crate::{ButtonAnim, LetterAnim};
use bevy::ecs::system::BoxedSystem;
use bevy::ecs::{lifecycle::HookContext, world::DeferredWorld};
use bevy::prelude::*;

pub const BUTTON_SIZE: u32 = 64;

impl PartialEq for ButtonAnim {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl ButtonAnim {
    fn get_color(&self) -> Color {
        match self {
            Self::Idle => Palatte::Snow.to_color(),
            Self::Hover => Palatte::Mist.to_color(),
            Self::Press => Palatte::Twilight.to_color(),
            Self::Disabled => Palatte::Wine.to_color(),
        }
    }
}

#[derive(Component)]
pub struct ButtonLetterAnimChild;

#[derive(Component)]
#[component(on_add = on_add_button)]
pub struct AnimButton {
    pub on_press: Option<BoxedSystem<(), ()>>,
    pub on_release: Option<BoxedSystem<(), ()>>,
    pub disabled_system: Option<BoxedSystem<(), bool>>,
    letter: LetterAnim,
    last_state: ButtonAnim,
    /// Set to true to disable the button. Can be set directly or via `disabled_system`.
    pub is_disabled: bool,
    pub trigger_press_this_frame: bool,
    pub trigger_release_this_frame: bool,
}

impl AnimButton {
    pub fn new(letter: LetterAnim) -> Self {
        Self {
            on_press: None,
            on_release: None,
            disabled_system: None,
            letter,
            last_state: ButtonAnim::Idle,
            is_disabled: false,
            trigger_press_this_frame: false,
            trigger_release_this_frame: false,
        }
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
}

fn on_add_button(mut world: DeferredWorld, hook: HookContext) {
    let button = world.get::<AnimButton>(hook.entity).unwrap();
    let letter = button.letter;
    let entity = hook.entity;

    world
        .commands()
        .entity(entity)
        .insert(AnimMan::new(ButtonAnim::Idle));

    world.commands().entity(entity).with_children(|parent| {
        parent.spawn((
            ButtonLetterAnimChild,
            AnimMan::new(letter),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Visibility::Inherited,
        ));
    });
}

fn update_button_disabled_state(world: &mut World) {
    let mut buttons_to_check = Vec::new();
    {
        let mut query = world.query::<(Entity, &AnimButton)>();
        for (entity, button) in query.iter(world) {
            if button.disabled_system.is_some() {
                buttons_to_check.push(entity);
            }
        }
    }

    for entity in buttons_to_check {
        let system_opt = world
            .get_mut::<AnimButton>(entity)
            .expect("Button entity should exist")
            .disabled_system
            .take();

        if let Some(mut disabled_system) = system_opt {
            disabled_system.initialize(world);
            let is_disabled = disabled_system
                .run((), world)
                .expect("Button disabled_system should run successfully");
            disabled_system.apply_deferred(world);

            let mut button = world
                .get_mut::<AnimButton>(entity)
                .expect("Button entity should exist");
            button.is_disabled = is_disabled;
            button.disabled_system = Some(disabled_system);
        }
    }
}

fn update_button_mouse_state(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut buttons: Query<(&GlobalTransform, &mut AnimButton, &mut AnimMan<ButtonAnim>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        for (_, mut button, mut anim) in buttons.iter_mut() {
            if !button.is_disabled {
                let new_state = ButtonAnim::Idle;
                button.last_state = new_state;
                anim.set(new_state);
            }
        }
        return;
    };

    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let lmb_pressed = mouse.pressed(MouseButton::Left);

    for (transform, mut button, mut anim) in buttons.iter_mut() {
        if button.is_disabled {
            if button.last_state != ButtonAnim::Disabled {
                button.last_state = ButtonAnim::Disabled;
                anim.set(ButtonAnim::Disabled);
            }
            continue;
        }

        let button_pos = transform.translation().truncate();
        let half_size = BUTTON_SIZE as f32 / 2.0;

        let is_over = world_pos.x >= button_pos.x - half_size
            && world_pos.x <= button_pos.x + half_size
            && world_pos.y >= button_pos.y - half_size
            && world_pos.y <= button_pos.y + half_size;

        let new_state = if !is_over {
            ButtonAnim::Idle
        } else if lmb_pressed {
            ButtonAnim::Press
        } else {
            ButtonAnim::Hover
        };

        if new_state == ButtonAnim::Press && button.last_state != ButtonAnim::Press {
            button.trigger_press_this_frame = true;
        }

        if button.last_state == ButtonAnim::Press && new_state == ButtonAnim::Hover {
            button.trigger_release_this_frame = true;
        }

        if new_state != button.last_state {
            button.last_state = new_state;
            anim.set(new_state);
        }
    }
}

fn update_letter_color(
    button_query: Query<(&AnimButton, &Children)>,
    mut letter_query: Query<&mut AnimMan<LetterAnim>, With<ButtonLetterAnimChild>>,
) {
    for (button, children) in button_query.iter() {
        let color = button.last_state.get_color();

        for child in children.iter() {
            if let Ok(mut letter_anim) = letter_query.get_mut(child) {
                letter_anim.set_color(color);
            }
        }
    }
}

fn react_to_triggers(world: &mut World) {
    let mut press_callbacks = Vec::new();
    let mut release_callbacks = Vec::new();

    {
        let mut query = world.query::<(Entity, &AnimButton)>();
        for (entity, button) in query.iter(world) {
            if button.trigger_press_this_frame && button.on_press.is_some() {
                press_callbacks.push(entity);
            }
            if button.trigger_release_this_frame && button.on_release.is_some() {
                release_callbacks.push(entity);
            }
        }
    }

    {
        let mut query = world.query::<&mut AnimButton>();
        for mut button in query.iter_mut(world) {
            button.trigger_press_this_frame = false;
            button.trigger_release_this_frame = false;
        }
    }

    for entity in press_callbacks {
        let system_opt = world
            .get_mut::<AnimButton>(entity)
            .expect("Button entity should exist")
            .on_press
            .take();

        if let Some(mut press_system) = system_opt {
            press_system.initialize(world);
            let _ = press_system.run((), world);
            press_system.apply_deferred(world);

            let mut button = world
                .get_mut::<AnimButton>(entity)
                .expect("Button entity should exist");
            button.on_press = Some(press_system);
        }
    }

    for entity in release_callbacks {
        let system_opt = world
            .get_mut::<AnimButton>(entity)
            .expect("Button entity should exist")
            .on_release
            .take();

        if let Some(mut release_system) = system_opt {
            release_system.initialize(world);
            let _ = release_system.run((), world);
            release_system.apply_deferred(world);

            let mut button = world
                .get_mut::<AnimButton>(entity)
                .expect("Button entity should exist");
            button.on_release = Some(release_system);
        }
    }
}

pub fn button_plugin_fn(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_button_disabled_state,
            update_button_mouse_state,
            update_letter_color,
            react_to_triggers,
        )
            .chain(),
    );
}
