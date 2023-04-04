use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::game::side::{powerup, Blank, Powerup, Side, SideActivateEvent};

pub fn init(app: &mut App) {
    app.add_system(jump_effect).add_system(jump_powerup);
}

#[derive(Component)]
pub struct Effect;

fn jump_effect(
    mut parents: Query<(&Transform, &mut Velocity)>,
    sides: Query<&Side, With<Effect>>,
    mut events: EventReader<SideActivateEvent>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        if let SideActivateEvent::Activated(side) = *event {
            let Ok(side) = sides.get(side) else { continue; };
            let Ok((parent_transform, mut parent_velocity)) = parents.get_mut(side.parent) else { continue };
            let normal = -parent_transform
                .with_translation(Vec3::ZERO)
                .mul_transform(side.transform)
                .transform_point(Vec3::ZERO)
                .xy();
            let vel_change = -normal * Vec2::dot(normal, parent_velocity.linvel) + normal * 30.0;
            parent_velocity.linvel += vel_change;
            audio.play(asset_server.load("hehehe.ogg"));
        }
    }
}

fn jump_powerup(
    mut commands: Commands,
    sides: Query<(With<Side>, With<Blank>)>,
    powerups: Query<(With<Powerup>, With<Effect>)>,
    mut events: EventReader<powerup::Event>,

    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        if !sides.contains(event.side) {
            continue;
        }
        if !powerups.contains(event.powerup) {
            continue;
        }
        commands.entity(event.powerup).despawn();
        commands.entity(event.side).insert(Effect).remove::<Blank>();

        // TODO: different system?
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
