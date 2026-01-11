use bevy::prelude::*;
use bits::prelude::*;

#[derive(Anim, Default, Clone, Copy, Debug)]
#[file("assets/bg/bg_world.aseprite")]
#[exclude_prefix("_")]
pub enum BgAnim {
    #[default]
    Idle,
}

#[derive(Anim, Default, Clone, Copy, Debug)]
#[file("assets/bg/star.aseprite")]
pub enum StarAnim {
    #[default]
    S1Small,
    S1Delta,
    S1Big,
    S2Small,
    S2Delta,
    S2Big,
    S3Small,
    S3Delta,
    S3Big,
    ShootSexy,
    ShootDownrish,
    ShootDown,
}
