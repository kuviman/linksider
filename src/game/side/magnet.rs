use bevy::utils::HashMap;

use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.register_side_effect::<Magnet>("MagnetPower");
        app.add_system(
            attach_to_walls
                .before(falling_system)
                .after(detect_side_effect::<Magnet>),
        );
    }
}

#[derive(Debug, Default, Component)]
pub struct Magnet;

impl SideEffect for Magnet {
    fn active_side() -> bool {
        true
    }
    fn active_above() -> bool {
        true
    }
}

fn attach_to_walls(
    players: Query<(Entity, &Rotation), With<Player>>,
    mut events: EventReader<SideEffectEvent<Magnet>>,
    mut commands: Commands,
) {
    for (player, _) in players.iter() {
        commands.entity(player).remove::<OverrideGravity>();
    }
    let mut go = HashMap::<Entity, Vec<IVec2>>::new();
    for event in events.iter() {
        if let Ok((_, player_rotation)) = players.get(event.player) {
            go.entry(event.player)
                .or_default()
                .push(side_vec(player_rotation.0, event.side));
        }
    }
    for (entity, gravities) in go {
        commands.entity(entity).insert(OverrideGravity(gravities));
    }
}
