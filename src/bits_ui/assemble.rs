use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use rand::{Rng, thread_rng};

fn cubic_ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[derive(Component, Debug, Reflect, Clone)]
struct Pixel {
    transform_start: Transform,
    transform_end: Transform,
    color_start: Color,
    color_end: Color,
}

impl Pixel {
    fn new(transform_start: Transform, transform_end: Transform) -> Self {
        Self {
            transform_start,
            transform_end,
            color_start: Color::srgba(0.0, 0.0, 0.0, 0.0),
            color_end: Color::srgba(1.0, 1.0, 1.0, 1.0),
        }
    }

    fn bundle(self) -> impl Bundle {
        (
            Sprite {
                color: self.color_start,
                ..default()
            },
            self.transform_start,
            self,
        )
    }
}

#[derive(Component)]
#[relationship(relationship_target = Assemble)]
pub struct PixelOf(Entity);

#[derive(Component)]
#[relationship_target(relationship = PixelOf, linked_spawn)]
#[component(on_add = on_add_assemble)]
pub struct Assemble {
    starting_lifespan: f32,
    lifespan: Option<f32>,
    min_radius: u32,
    max_radius: u32,
    pixel_locations: Vec<IVec2>,
    #[relationship]
    pixels: Vec<Entity>,
}

impl Assemble {
    pub fn new() -> Self {
        Self {
            starting_lifespan: 1.0,
            lifespan: Some(1.0),
            min_radius: 50,
            max_radius: 100,
            pixel_locations: vec![],
            pixels: vec![],
        }
    }

    pub fn with_pixel_locations(mut self, locations: Vec<IVec2>) -> Self {
        self.pixel_locations = locations;
        self
    }

    pub fn with_min_radius(mut self, min_radius: u32) -> Self {
        self.min_radius = min_radius;
        self
    }

    pub fn with_max_radius(mut self, max_radius: u32) -> Self {
        self.max_radius = max_radius;
        self
    }

    pub fn with_lifespan(mut self, lifespan: f32) -> Self {
        self.starting_lifespan = lifespan;
        self.lifespan = Some(lifespan);
        self
    }
}

fn on_add_assemble(mut world: DeferredWorld, hook: HookContext) {
    let assemble = world.get::<Assemble>(hook.entity).unwrap();
    let pixel_locations = assemble.pixel_locations.clone();
    let min_radius = assemble.min_radius;
    let max_radius = assemble.max_radius;

    let transform = world.get::<Transform>(hook.entity).cloned().unwrap();

    let get_transform = |offset: IVec2| {
        transform.with_translation(transform.translation + offset.extend(0).as_vec3())
    };
    let random_sign = || {
        if thread_rng().gen_bool(0.5) { 1 } else { -1 }
    };

    let entity = hook.entity;
    world.commands().entity(entity).with_children(|commands| {
        for pixel_location in &pixel_locations {
            let distance = thread_rng().gen_range(min_radius..max_radius);
            let x_dist = random_sign() * thread_rng().gen_range(0..distance) as i32;
            let y_dist = random_sign() * (distance as i32 - x_dist.abs());

            commands.spawn((
                Pixel::new(
                    get_transform(IVec2::new(x_dist, y_dist)),
                    get_transform(*pixel_location),
                )
                .bundle(),
                PixelOf(entity),
                Visibility::Inherited,
            ));
        }
    });
}

fn update_assembles(
    time: Res<Time>,
    mut commands: Commands,
    mut assemble_q: Query<(Entity, &mut Assemble)>,
    mut pixel_q: Query<(&Pixel, &mut Transform, &mut Sprite)>,
) {
    for (assemble_eid, mut assemble) in assemble_q.iter_mut() {
        if assemble.lifespan.is_none() {
            for pixel_eid in assemble.iter() {
                commands.entity(pixel_eid).despawn();
            }
            commands.entity(assemble_eid).remove::<Assemble>();
            continue;
        }

        let inner = assemble.lifespan.as_mut().unwrap();
        *inner -= time.delta_secs();
        if *inner < 0.0 {
            assemble.lifespan = None;
        }

        let lifespan_frac = (assemble.starting_lifespan - assemble.lifespan.unwrap_or(0.0))
            / assemble.starting_lifespan;
        let eased = cubic_ease_out(lifespan_frac);

        for pixel_eid in assemble.iter() {
            let (pixel, mut transform, mut sprite) = pixel_q.get_mut(pixel_eid).unwrap();

            transform.translation = pixel
                .transform_start
                .translation
                .lerp(pixel.transform_end.translation, eased);
            transform.rotation = pixel
                .transform_start
                .rotation
                .slerp(pixel.transform_end.rotation, eased);
            transform.scale = pixel
                .transform_start
                .scale
                .lerp(pixel.transform_end.scale, eased);

            sprite.color = pixel.color_start.mix(&pixel.color_end, eased);
        }
    }
}

pub(crate) fn assemble_simple_plugin_fn(app: &mut App) {
    app.add_systems(Update, update_assembles);
}
