use crate::bits_ui::anim::AnimMan;
use crate::bits_ui::colors::Palatte;
use crate::window::WINDOW_SIZE;
use crate::{BgAnim, StarAnim};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use rand::{Rng, thread_rng};

const BG_Z: f32 = -10.0;
const STAR_Z: f32 = -11.0;

const STAR_COLORS: [Palatte; 6] = [
    Palatte::Snow,
    Palatte::Mist,
    Palatte::Blush,
    Palatte::Sakura,
    Palatte::Cream,
    Palatte::Honey,
];

#[derive(Clone, Reflect, Debug)]
pub struct PhaseTiming {
    pub min_secs: f32,
    pub max_secs: f32,
    pub weight: f32,
}

impl PhaseTiming {
    pub fn new(min_secs: f32, max_secs: f32, weight: f32) -> Self {
        Self {
            min_secs,
            max_secs,
            weight,
        }
    }

    pub fn sample(&self) -> f32 {
        thread_rng().gen_range(self.min_secs..=self.max_secs)
    }
}

#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct BgSettings {
    pub star_count: u32,
    pub min_star_spacing: f32,
    pub vertical_gradient_power: f32,
    pub phase_timings: Vec<PhaseTiming>,
    pub transition_duration_multiplier: f32,
    pub bloom_base: f32,
    pub bloom_big: f32,
    pub bloom_shooting: f32,
    pub shooting_star_chance_per_second: f32,
}

impl Default for BgSettings {
    fn default() -> Self {
        Self {
            star_count: 64,
            min_star_spacing: 32.0,
            vertical_gradient_power: 2.0,
            phase_timings: vec![
                PhaseTiming::new(2.0, 5.0, 0.3),
                PhaseTiming::new(6.0, 15.0, 1.7),
            ],
            transition_duration_multiplier: 2.5,
            bloom_base: 2.0,
            bloom_big: 4.0,
            bloom_shooting: 8.0,
            shooting_star_chance_per_second: 3.0,
        }
    }
}

impl BgSettings {
    pub fn pick_phase_timing(&self) -> &PhaseTiming {
        let total_weight: f32 = self.phase_timings.iter().map(|t| t.weight).sum();
        if total_weight <= 0.0 || self.phase_timings.is_empty() {
            return &PhaseTiming {
                min_secs: 1.0,
                max_secs: 2.0,
                weight: 1.0,
            };
        }

        let mut roll = thread_rng().gen_range(0.0..total_weight);
        for timing in &self.phase_timings {
            roll -= timing.weight;
            if roll <= 0.0 {
                return timing;
            }
        }
        self.phase_timings.last().unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarStyle {
    S1,
    S2,
    S3,
}

impl StarStyle {
    pub fn random() -> Self {
        match thread_rng().gen_range(0..3) {
            0 => StarStyle::S1,
            1 => StarStyle::S2,
            _ => StarStyle::S3,
        }
    }

    pub fn anim_for_phase(&self, phase: StarPhase) -> StarAnim {
        match (self, phase) {
            (StarStyle::S1, StarPhase::Small) => StarAnim::S1Small,
            (StarStyle::S1, StarPhase::TransitionToBig) => StarAnim::S1Delta,
            (StarStyle::S1, StarPhase::Big) => StarAnim::S1Big,
            (StarStyle::S1, StarPhase::TransitionToSmall) => StarAnim::S1Delta,
            (StarStyle::S2, StarPhase::Small) => StarAnim::S2Small,
            (StarStyle::S2, StarPhase::TransitionToBig) => StarAnim::S2Delta,
            (StarStyle::S2, StarPhase::Big) => StarAnim::S2Big,
            (StarStyle::S2, StarPhase::TransitionToSmall) => StarAnim::S2Delta,
            (StarStyle::S3, StarPhase::Small) => StarAnim::S3Small,
            (StarStyle::S3, StarPhase::TransitionToBig) => StarAnim::S3Delta,
            (StarStyle::S3, StarPhase::Big) => StarAnim::S3Big,
            (StarStyle::S3, StarPhase::TransitionToSmall) => StarAnim::S3Delta,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarPhase {
    Small,
    TransitionToBig,
    Big,
    TransitionToSmall,
}

impl StarPhase {
    pub fn next(self) -> Self {
        match self {
            StarPhase::Small => StarPhase::TransitionToBig,
            StarPhase::TransitionToBig => StarPhase::Big,
            StarPhase::Big => StarPhase::TransitionToSmall,
            StarPhase::TransitionToSmall => StarPhase::Small,
        }
    }

    pub fn is_bright(self) -> bool {
        matches!(self, StarPhase::Big | StarPhase::TransitionToBig)
    }

    pub fn is_transition(self) -> bool {
        matches!(
            self,
            StarPhase::TransitionToBig | StarPhase::TransitionToSmall
        )
    }
}

#[derive(Component)]
pub struct Star {
    pub style: StarStyle,
    pub phase: StarPhase,
    pub timer: f32,
    pub phase_duration: f32,
    pub base_color: Color,
}

impl Star {
    pub fn new(style: StarStyle, base_color: Color, initial_duration: f32) -> Self {
        Self {
            style,
            phase: StarPhase::Small,
            timer: initial_duration,
            phase_duration: initial_duration,
            base_color,
        }
    }

    pub fn bloom_multiplier(&self, settings: &BgSettings) -> f32 {
        if self.phase.is_bright() {
            settings.bloom_big
        } else {
            settings.bloom_base
        }
    }

    pub fn bloomed_color(&self, settings: &BgSettings) -> Color {
        let mult = self.bloom_multiplier(settings);
        let linear = self.base_color.to_linear();
        Color::linear_rgb(linear.red * mult, linear.green * mult, linear.blue * mult)
    }
}

#[derive(Component)]
#[component(on_add = on_add_bg_marker)]
pub struct BgMarker {
    shooting_star_accumulator: f32,
}

impl Default for BgMarker {
    fn default() -> Self {
        Self {
            shooting_star_accumulator: 0.0,
        }
    }
}

fn on_add_bg_marker(mut world: DeferredWorld, ctx: HookContext) {
    let settings = world
        .get_resource::<BgSettings>()
        .cloned()
        .unwrap_or_default();

    let entity = ctx.entity;

    world.commands().entity(entity).with_children(|parent| {
        parent.spawn((
            Name::new("Background"),
            AnimMan::new(BgAnim::Idle),
            Transform::from_xyz(0.0, 0.0, BG_Z),
            Visibility::Inherited,
        ));

        let star_positions = generate_star_positions(
            settings.star_count,
            settings.min_star_spacing,
            settings.vertical_gradient_power,
        );

        for pos in star_positions {
            let style = StarStyle::random();
            let base_color = random_star_color();
            let timing = settings.pick_phase_timing();
            let duration = timing.sample();

            let star = Star::new(style, base_color, duration);
            let bloomed_color = star.bloomed_color(&settings);
            let anim = style.anim_for_phase(StarPhase::Small);

            parent.spawn((
                Name::new("Star"),
                star,
                AnimMan::new(anim).with_color(bloomed_color),
                Transform::from_xyz(pos.x, pos.y, STAR_Z),
                Visibility::Inherited,
            ));
        }
    });
}

fn generate_star_positions(count: u32, min_spacing: f32, _gradient_power: f32) -> Vec<Vec2> {
    let half_size = WINDOW_SIZE as f32 / 2.0;
    let mut positions = Vec::with_capacity(count as usize);
    let mut attempts = 0;
    let max_attempts = count * 100;

    while positions.len() < count as usize && attempts < max_attempts {
        attempts += 1;

        let x = thread_rng().gen_range(-half_size..half_size);
        let y = thread_rng().gen_range(-half_size..half_size);

        let candidate = Vec2::new(x, y);

        let too_close = positions
            .iter()
            .any(|p: &Vec2| p.distance(candidate) < min_spacing);

        if !too_close {
            positions.push(candidate);
        }
    }

    positions
}

fn random_star_color() -> Color {
    let idx = thread_rng().gen_range(0..STAR_COLORS.len());
    STAR_COLORS[idx].to_color()
}

fn star_flicker_system(
    time: Res<Time>,
    settings: Res<BgSettings>,
    mut query: Query<(&mut Star, &mut AnimMan<StarAnim>)>,
) {
    for (mut star, mut anim) in query.iter_mut() {
        star.timer -= time.delta_secs();

        if star.timer <= 0.0 {
            star.phase = star.phase.next();

            let timing = settings.pick_phase_timing();
            let mut duration = timing.sample();

            // Transition phases (delta frames) last longer
            if star.phase.is_transition() {
                duration *= settings.transition_duration_multiplier;
            }

            star.phase_duration = duration;
            star.timer = duration;

            let new_anim = star.style.anim_for_phase(star.phase);
            anim.set(new_anim);
        }

        let bloomed_color = star.bloomed_color(&settings);
        anim.set_color(bloomed_color);
    }
}

fn shooting_star_spawner_system(
    time: Res<Time>,
    settings: Res<BgSettings>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut BgMarker)>,
) {
    let half_size = WINDOW_SIZE as f32 / 2.0;

    for (entity, mut bg_marker) in query.iter_mut() {
        bg_marker.shooting_star_accumulator += time.delta_secs();

        let chance = settings.shooting_star_chance_per_second * time.delta_secs();
        if thread_rng().gen_bool((chance as f64).min(1.0)) {
            let variants = [
                StarAnim::ShootSexy,
                StarAnim::ShootDownrish,
                StarAnim::ShootDown,
                StarAnim::ShootDownLong,
                StarAnim::ShootDiagonalLong,
            ];
            let variant = variants[thread_rng().gen_range(0..variants.len())];

            let x = thread_rng().gen_range(-half_size..half_size);
            let y = thread_rng().gen_range(-half_size..half_size);

            let base_color = random_star_color();
            let linear = base_color.to_linear();
            let mult = settings.bloom_shooting;
            let bloomed_color =
                Color::linear_rgb(linear.red * mult, linear.green * mult, linear.blue * mult);

            let flip_x = thread_rng().gen_bool(0.5);
            let flip_y = thread_rng().gen_bool(0.5);

            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Name::new("Shooting Star"),
                    AnimMan::new(variant)
                        .with_color(bloomed_color)
                        .with_flip_x(flip_x)
                        .with_flip_y(flip_y),
                    Transform::from_xyz(x, y, STAR_Z),
                    Visibility::Inherited,
                ));
            });
        }
    }
}

pub fn bg_plugin_fn(app: &mut App) {
    app.init_resource::<BgSettings>()
        .register_type::<BgSettings>()
        .register_type::<PhaseTiming>()
        .add_systems(Update, (star_flicker_system, shooting_star_spawner_system));
}
