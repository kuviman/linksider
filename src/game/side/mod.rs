use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_ecs_ldtk::EntityInstance;
use bevy_rapier2d::prelude::*;

pub mod effects;
pub mod powerup;

pub use powerup::Powerup;

#[derive(Component)]
pub struct HasSides(pub usize);

impl Default for HasSides {
    fn default() -> Self {
        Self(4)
    }
}

pub fn init(app: &mut App) {
    app.add_system(side_setup)
        .add_system(side_trigger_collision_counter)
        .add_system(side_activation)
        .add_event::<SideActivateEvent>();
    powerup::init(app);
    effects::jump::init(app);
    effects::slide::init(app);
}

fn side_setup(
    parents: Query<(Entity, &EntityInstance, &HasSides), Added<HasSides>>,
    mut commands: Commands,
) {
    for (parent, entity_instance, &HasSides(sides)) in parents.iter() {
        let tile = entity_instance.tile.as_ref().unwrap();
        for i in 0..sides {
            let side = commands
                .spawn((
                    Collider::cuboid(0.5, 0.2),
                    TransformBundle::from_transform(
                        Transform::from_scale(Vec3::new(tile.w as f32, tile.h as f32, 1.0))
                            * Transform::from_rotation(Quat::from_rotation_z(
                                i as f32 * 2.0 * PI / sides as f32,
                            ))
                            * Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
                    ),
                    Side,
                    TriggerCollisionNumber(0),
                    Blank,
                    Sensor,
                    ActiveEvents::COLLISION_EVENTS,
                    ActiveCollisionTypes::all(),
                    Name::new(format!("Side {i}")),
                ))
                .id();
            commands.entity(parent).add_child(side);
        }
    }
}

#[derive(Component)]
pub struct Blank;

#[derive(Component)]
pub struct Side;

#[derive(Default, Component)]
pub struct Trigger;

#[derive(Component)]
pub struct Active;

#[derive(Component)]
struct TriggerCollisionNumber(i32);

fn side_trigger_collision_counter(
    mut sides: Query<&mut TriggerCollisionNumber, With<Side>>,
    side_triggers: Query<Entity, With<Trigger>>,
    mut collisions: EventReader<CollisionEvent>,
) {
    let mut process = |a, b, inc: i32| {
        let mut check = |side, trigger| {
            let Ok(mut collision_number) = sides.get_mut(side) else {
                return;
            };
            if !side_triggers.contains(trigger) {
                return;
            }
            collision_number.0 += inc;
        };
        check(a, b);
        check(b, a);
    };
    for event in collisions.iter() {
        match *event {
            CollisionEvent::Started(a, b, _) => {
                process(a, b, 1);
            }
            CollisionEvent::Stopped(a, b, _) => {
                process(a, b, -1);
            }
        }
    }
}

#[derive(Clone, Debug)]
enum SideActivateEvent {
    Activated(Entity),
    Deactivated(Entity),
}

impl SideActivateEvent {
    fn side(&self) -> Entity {
        match *self {
            SideActivateEvent::Activated(side) | SideActivateEvent::Deactivated(side) => side,
        }
    }
}

fn side_activation(
    sides: Query<
        (Entity, Option<&Active>, &TriggerCollisionNumber),
        (With<Side>, Changed<TriggerCollisionNumber>),
    >,
    mut events: EventWriter<SideActivateEvent>,
    mut commands: Commands,
) {
    for (entity, active, number) in sides.iter() {
        let should_be_active = number.0 != 0;
        let actually_active = active.is_some();
        if !should_be_active && actually_active {
            commands.entity(entity).remove::<Active>();
            events.send(SideActivateEvent::Deactivated(entity));
        }
        if should_be_active && !actually_active {
            commands.entity(entity).insert(Active);
            events.send(SideActivateEvent::Activated(entity));
        }
    }
}
