use bevy::{ecs::system::BoxedSystem, prelude::*};

#[derive(Component)]
#[require(Name)]
pub struct FlexSimple {
    width: Val,
    height: Val,
    flex_direction: FlexDirection,
    align_items: AlignItems,
    justify_content: JustifyContent,
    pub visibility_system: Option<BoxedSystem<(), bool>>,
    current_visible: bool,
}

impl FlexSimple {
    pub fn new() -> Self {
        Self {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            visibility_system: None,
            current_visible: true,
        }
    }

    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.flex_direction = direction;
        self
    }

    pub fn with_size(mut self, width: Val, height: Val) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_visibility_system<M>(mut self, system: impl IntoSystem<(), bool, M>) -> Self {
        self.visibility_system = Some(Box::new(IntoSystem::into_system(system)));
        self
    }

    pub fn bundle(self) -> impl Bundle {
        (
            Name::new("FlexSimple"),
            Node {
                width: self.width,
                height: self.height,
                flex_direction: self.flex_direction,
                align_items: self.align_items,
                justify_content: self.justify_content,
                display: if self.current_visible {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
            self,
        )
    }
}

fn flex_simple_visibility_system(world: &mut World) {
    let mut flexes_to_check = Vec::new();
    {
        let mut query = world.query::<(Entity, &FlexSimple)>();
        for (entity, flex_simple) in query.iter(world) {
            if flex_simple.visibility_system.is_some() {
                flexes_to_check.push(entity);
            }
        }
    }

    for entity in flexes_to_check {
        let system_opt = world
            .get_mut::<FlexSimple>(entity)
            .expect("FlexSimple entity should exist")
            .visibility_system
            .take();

        if let Some(mut visibility_system) = system_opt {
            visibility_system.initialize(world);
            let new_visible = visibility_system
                .run((), world)
                .expect("FlexSimple visibility_system should run successfully");
            visibility_system.apply_deferred(world);

            let mut flex_simple = world
                .get_mut::<FlexSimple>(entity)
                .expect("FlexSimple entity should exist");

            if new_visible != flex_simple.current_visible {
                flex_simple.current_visible = new_visible;
                flex_simple.visibility_system = Some(visibility_system);

                if let Some(mut node) = world.get_mut::<Node>(entity) {
                    node.display = if new_visible {
                        Display::Flex
                    } else {
                        Display::None
                    };
                }
            } else {
                flex_simple.visibility_system = Some(visibility_system);
            }
        }
    }
}

pub fn flex_simple_plugin_fn(app: &mut App) {
    app.add_systems(FixedUpdate, flex_simple_visibility_system);
}
