use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::game::side::{self, powerup, Blank, Powerup, Side, SideActivateEvent};

pub fn init(app: &mut App) {
    app.add_system(activation)
        .add_system(continious_effect)
        .add_system(powerup);
}

#[derive(Default, Component)]
pub struct Effect;

#[derive(Component)]
struct JumpTimer(f32);

fn activation(
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
            let vel_change =
                -direction * Vec2::dot(direction, parent_velocity.linvel) - direction * 100.0;
            parent_velocity.linvel += vel_change;
            commands.entity(event.side()).insert(JumpTimer(0.0));
            audio.play(asset_server.load("jump.ogg"));
        }
    }
}

fn continious_effect(
    time: Res<Time>,
    mut sides: Query<(Entity, &Parent, &Transform, &mut JumpTimer)>,
    mut parents: Query<(&Transform, &mut Velocity)>,
    mut commands: Commands,
) {
    for (side, parent, transform, mut jump_timer) in sides.iter_mut() {
        let Ok((parent_transform, mut parent_velocity)) = parents.get_mut(parent.get()) else { continue };
        let direction = (parent_transform.rotation * transform.rotation * Vec3::Y).xy();
        parent_velocity.linvel += -direction * time.delta_seconds() * 200.0;
        jump_timer.0 += time.delta_seconds();
        if jump_timer.0 > 1.0 {
            commands.entity(side).remove::<JumpTimer>();
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
