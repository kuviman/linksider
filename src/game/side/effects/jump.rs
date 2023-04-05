use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::game::side::{powerup, Blank, Powerup, Side, SideActivateEvent};

pub fn init(app: &mut App) {
    app.add_system(jump_effect).add_system(jump_powerup);
}

#[derive(Default, Component)]
pub struct Effect;

fn jump_effect(
    mut parents: Query<(&Transform, &mut Velocity)>,
    sides: Query<(&Parent, &Transform), (With<Side>, With<Effect>)>,
    mut events: EventReader<SideActivateEvent>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        if let SideActivateEvent::Activated(side) = *event {
            let Ok((parent, transform)) = sides.get(side) else { continue; };
            let Ok((parent_transform, mut parent_velocity)) = parents.get_mut(parent.get()) else { continue };
            let direction = (parent_transform.rotation * transform.rotation * Vec3::Y).xy();
            let vel_change =
                -direction * Vec2::dot(direction, parent_velocity.linvel) - direction * 200.0;
            parent_velocity.linvel += vel_change;
            audio.play(asset_server.load("woohoo.ogg"));
        }
    }
}

fn jump_powerup(
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
