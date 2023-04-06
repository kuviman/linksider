use bevy::prelude::*;

#[derive(Debug, serde::Deserialize, Resource, Reflect)]
pub struct JumpEffectConfig {
    pub impulse: f32,
    pub force: f32,
    pub particle_vel: f32,
}

#[derive(Debug, serde::Deserialize, Resource, Reflect)]
pub struct SlideEffectConfig {
    pub stick_force: f32,
    pub move_force: f32,
}

#[derive(Debug, serde::Deserialize, Resource, Reflect)]
pub struct Config {
    pub player_rotation_speed: f32,
    pub player_rotation_accel: f32,
    pub jump_effect: JumpEffectConfig,
    pub slide_effect: SlideEffectConfig,
}
