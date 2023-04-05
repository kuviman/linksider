use super::Side;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Default, Component)]
pub struct Powerup;

pub struct Event {
    pub side: Entity,
    pub powerup: Entity,
}

pub fn init(app: &mut App) {
    app.add_system(collect_powerup).add_event::<Event>();
}

fn collect_powerup(
    sides: Query<Entity, With<Side>>,
    powerups: Query<Entity, With<Powerup>>,
    mut collisions: EventReader<CollisionEvent>,
    mut events: EventWriter<Event>,
) {
    for event in collisions.iter() {
        if let CollisionEvent::Started(a, b, _) = *event {
            let mut check = |a, b| {
                if !sides.contains(a) {
                    return;
                }
                if !powerups.contains(b) {
                    return;
                }
                events.send(Event {
                    side: a,
                    powerup: b,
                });
            };
            check(a, b);
            check(b, a);
        }
    }
}
