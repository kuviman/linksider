use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub mod effects;
pub mod powerup;

pub use powerup::Powerup;

pub fn init(app: &mut App) {
    app.add_system(side_activation)
        .add_system(update_side_transforms)
        .add_event::<SideActivateEvent>();
    powerup::init(app);
    effects::jump::init(app);
}

#[derive(Component)]
pub struct Blank;

#[derive(Component)]
pub struct Trigger;

#[derive(Component)]
pub struct Side {
    pub transform: Transform,
    pub parent: Entity,
}

fn update_side_transforms(
    mut sides: Query<(&mut Transform, &Side)>,
    parents: Query<&Transform, Without<Side>>,
) {
    for (mut side_transform, side) in sides.iter_mut() {
        if let Ok(parent_transform) = parents.get(side.parent) {
            *side_transform = parent_transform.mul_transform(side.transform);
        }
    }
}

#[derive(Debug)]
enum SideActivateEvent {
    Activated(Entity),
    Deactivated(Entity),
}

fn side_activation(
    sides: Query<Entity, With<Side>>,
    side_triggers: Query<Entity, With<Trigger>>,
    mut collisions: EventReader<CollisionEvent>,
    mut events: EventWriter<SideActivateEvent>,
) {
    let mut process = |a, b, f: fn(Entity) -> SideActivateEvent| {
        let mut check = |a, b| {
            if !sides.contains(a) {
                return;
            }
            if !side_triggers.contains(b) {
                return;
            }
            events.send(f(a));
        };
        check(a, b);
        check(b, a);
    };
    for event in collisions.iter() {
        match *event {
            CollisionEvent::Started(a, b, _) => {
                process(a, b, SideActivateEvent::Activated);
            }
            CollisionEvent::Stopped(a, b, _) => {
                process(a, b, SideActivateEvent::Deactivated);
            }
        }
    }
}
