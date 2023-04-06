use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::game::{
    config::Config,
    side::{self, powerup, Blank, Powerup, Side, SideActivateEvent},
};

pub fn init(app: &mut App) {
    app.add_system(activation)
        .add_system(continious_effect)
        .add_system(powerup)
        .add_system(particle_system);
}

#[derive(Default, Component)]
pub struct Effect;

#[derive(Component)]
struct JumpTimer {
    time: f32,
    timer: Timer,
}

fn activation(
    config: Res<Config>,
    mut parents: Query<(&Transform, &mut Velocity)>,
    sides: Query<(&Parent, &Transform), (With<Side>, With<Effect>)>,
    mut events: EventReader<SideActivateEvent>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for event in events.iter() {
        if let SideActivateEvent::Activated(side) = *event {
            let Ok((parent, transform)) = sides.get(side) else { continue; };
            let Ok((parent_transform, mut parent_velocity)) = parents.get_mut(parent.get()) else { continue };
            let direction = (parent_transform.rotation * transform.rotation * Vec3::Y).xy();
            let vel_change = -direction * Vec2::dot(direction, parent_velocity.linvel)
                - direction * config.jump_effect.impulse;
            parent_velocity.linvel += vel_change;
            commands.entity(event.side()).insert(JumpTimer {
                time: 0.0,
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            });
            audio.play(asset_server.load("jump.ogg"));
        }
    }
}

#[derive(Component)]
struct Particle(Timer);

fn continious_effect(
    config: Res<Config>,
    time: Res<Time>,
    mut sides: Query<(
        Entity,
        &Parent,
        &Transform,
        &GlobalTransform,
        &mut JumpTimer,
    )>,
    mut parents: Query<(&Transform, &mut Velocity)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (side, parent, transform, global_transform, mut jump_timer) in sides.iter_mut() {
        let Ok((parent_transform, mut parent_velocity)) = parents.get_mut(parent.get()) else { continue };
        let direction = (parent_transform.rotation * transform.rotation * Vec3::Y).xy();
        parent_velocity.linvel += -direction * time.delta_seconds() * config.jump_effect.force;
        jump_timer.time += time.delta_seconds();
        if jump_timer.timer.tick(time.delta()).just_finished() {
            commands.spawn((
                SpriteBundle {
                    transform: Transform::from_translation(global_transform.translation()),
                    texture: asset_server.load("jump_particle.png"),
                    ..default()
                },
                RigidBody::KinematicVelocityBased,
                Velocity::linear(
                    parent_velocity.linvel + direction * config.jump_effect.particle_vel,
                ),
                Particle(Timer::from_seconds(1.0, TimerMode::Once)),
            ));
        }
        if jump_timer.time > 1.0 {
            commands.entity(side).remove::<JumpTimer>();
        }
    }
}

fn particle_system(
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Sprite, &mut Particle)>,
    mut commands: Commands,
) {
    for (entity, mut sprite, mut particle) in particles.iter_mut() {
        let opacity = 1.0 - particle.0.elapsed_secs() / particle.0.duration().as_secs_f32();
        sprite.color.set_a(opacity);
        if particle.0.tick(time.delta()).finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn powerup(
    mut commands: Commands,
    mut sides: Query<&mut Collider, (With<Side>, With<Blank>)>,
    powerups: Query<(With<Powerup>, With<Effect>)>,
    mut events: EventReader<powerup::Event>,

    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        let Ok(mut collider) = sides.get_mut(event.side) else { continue };
        if !powerups.contains(event.powerup) {
            continue;
        }
        commands.entity(event.powerup).despawn();
        commands.entity(event.side).insert(Effect).remove::<Blank>();

        let mut cuboid = collider.as_cuboid_mut().unwrap();
        cuboid.sed_half_extents({
            let mut v = cuboid.half_extents();
            v /= 100.0;
            v
        });
        commands.entity(event.side).insert((
            Sprite {
                custom_size: Some(Vec2::new(1.0, 0.25)),
                ..default()
            },
            asset_server.load::<Image, _>("side_effects/jump.png"),
            Visibility::default(),
            ComputedVisibility::default(),
        ));
    }
}
