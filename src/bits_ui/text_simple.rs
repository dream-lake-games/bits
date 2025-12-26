use bevy::{ecs::system::BoxedSystem, prelude::*};

#[derive(Component)]
#[require(BackgroundColor, BorderColor, Name)]
pub struct TextSimple {
    font_size: f32,
    width: Val,
    height: Val,
    font_color: Color,
    bg_color: Color,
    border_color: Color,
    border_width: Val,
    justify: Justify,
    pub text_system: Option<BoxedSystem<(), String>>,
    current_text: String,
}

impl TextSimple {
    pub fn h1(text: impl Into<String>) -> Self {
        Self {
            font_size: 48.0,
            width: Val::Auto,
            height: Val::Auto,
            font_color: Color::srgb(0.9, 0.9, 0.9),
            bg_color: Color::NONE,
            border_color: Color::NONE,
            border_width: Val::Px(0.0),
            justify: Justify::Center,
            text_system: None,
            current_text: text.into(),
        }
    }

    pub fn p(text: impl Into<String>) -> Self {
        Self {
            font_size: 16.0,
            width: Val::Auto,
            height: Val::Auto,
            font_color: Color::srgb(0.9, 0.9, 0.9),
            bg_color: Color::NONE,
            border_color: Color::NONE,
            border_width: Val::Px(0.0),
            justify: Justify::Center,
            text_system: None,
            current_text: text.into(),
        }
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_size(mut self, width: Val, height: Val) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_font_color(mut self, color: Color) -> Self {
        self.font_color = color;
        self
    }

    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    pub fn with_border(mut self, width: Val, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn with_justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    pub fn with_text_system<M>(mut self, system: impl IntoSystem<(), String, M> + 'static) -> Self {
        self.text_system = Some(Box::new(IntoSystem::into_system(system)));
        self
    }

    pub fn bundle(self) -> impl Bundle {
        let font_size = self.font_size;
        let width = self.width;
        let height = self.height;
        let font_color = self.font_color;
        let bg_color = self.bg_color;
        let border_color = self.border_color;
        let border_width = self.border_width;
        let justify = self.justify;
        let text = self.current_text.clone();

        (
            Node {
                width,
                height,
                border: UiRect::all(border_width),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(border_color),
            children![(
                Text::new(text),
                TextColor(font_color),
                TextFont::default().with_font_size(font_size),
                TextLayout::new_with_justify(justify),
            )],
            self,
        )
    }
}

fn text_simple_system(world: &mut World) {
    let mut texts_to_check = Vec::new();
    {
        let mut query = world.query::<(Entity, &TextSimple)>();
        for (entity, text_simple) in query.iter(world) {
            if text_simple.text_system.is_some() {
                texts_to_check.push(entity);
            }
        }
    }

    for entity in texts_to_check {
        let system_opt = world
            .get_mut::<TextSimple>(entity)
            .expect("TextSimple entity should exist")
            .text_system
            .take();

        if let Some(mut text_system) = system_opt {
            text_system.initialize(world);
            let new_text = text_system
                .run((), world)
                .expect("TextSimple text_system should run successfully");
            text_system.apply_deferred(world);

            let mut text_simple = world
                .get_mut::<TextSimple>(entity)
                .expect("TextSimple entity should exist");

            if new_text != text_simple.current_text {
                text_simple.current_text = new_text.clone();
                text_simple.text_system = Some(text_system);

                if let Some(children) = world.get::<Children>(entity) {
                    if let Some(&child) = children.first() {
                        if let Some(mut text) = world.get_mut::<Text>(child) {
                            **text = new_text;
                        }
                    }
                }
            } else {
                text_simple.text_system = Some(text_system);
            }
        }
    }
}

pub fn text_simple_plugin_fn(app: &mut App) {
    app.add_systems(Update, text_simple_system);
}
