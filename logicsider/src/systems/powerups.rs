use super::*;

pub fn process(state: &mut GameState) -> Vec<CollectedPowerup> {
    let mut collected = Vec::new();
    for entity in &state.entities {
        for powerup in &state.powerups {
            if entity.pos.cell != powerup.pos.cell {
                continue;
            }
            let entity_side = entity.side_index(powerup.pos.angle);
            if entity.sides[entity_side].effect.is_none() {
                collected.push(CollectedPowerup {
                    entity: entity.id,
                    entity_side: entity_side,
                    powerup: powerup.id,
                })
            }
        }
    }
    for event in &collected {
        let powerup = state.powerups.remove(&event.powerup).unwrap();
        let prev_effect = state.entities.get_mut(&event.entity).unwrap().sides[event.entity_side]
            .effect
            .replace(powerup.effect);
        assert!(prev_effect.is_none());
    }
    collected
}
