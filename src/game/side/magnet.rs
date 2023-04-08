use super::*;

pub fn init(app: &mut App) {
    app.register_side_effect::<Magnet>("MagnetPower");
    app.add_system(
        attach_to_walls
            .before(falling_system)
            .after(detect_side_effect::<Magnet>),
    );
}

#[derive(Debug, Default, Component)]
pub struct Magnet;

impl SideEffect for Magnet {
    fn texture() -> &'static str {
        "side_effects/magnet.png"
    }
    fn active_side() -> bool {
        true
    }
    fn active_above() -> bool {
        true
    }
}

fn attach_to_walls(
    players: Query<Entity, With<Player>>,
    mut events: EventReader<SideEffectEvent<Magnet>>,
    mut commands: Commands,
) {
    for player in players.iter() {
        commands.entity(player).remove::<DisableGravity>();
    }
    for event in events.iter() {
        info!("{event:?}");
        commands.entity(event.player).insert(DisableGravity);
    }
}
