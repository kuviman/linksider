use super::*;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub r#continue: systems::magnet::ContinueConfig,
    pub continue_horizontal: bool,
    pub continue_when_magneted: bool,
}

#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ContinueConfig {
    Input,
    Always,
    Never,
}

pub fn entity_magneted_angles(
    state: &GameState,
    entity_id: Id,
) -> impl Iterator<Item = IntAngle> + '_ {
    effects::entity_active_effects(state, entity_id).flat_map(|(side, effect)| {
        if let Effect::Magnet = effect.deref() {
            Some(side)
        } else {
            None
        }
    })
}

pub fn continue_move(
    EntityMoveParams {
        state,
        config,
        entity_id,
        input,
        ..
    }: EntityMoveParams,
) -> Option<EntityMove> {
    if config.magnet.r#continue == ContinueConfig::Never {
        return None;
    }
    if entity_magneted_angles(state, entity_id).next().is_some()
        && !config.magnet.continue_when_magneted
    {
        return None;
    }
    let entity = state.entities.get(&entity_id).unwrap();
    let Some(EntityMove {
            used_input: prev_input,
            move_type: EntityMoveType::Magnet {
                magnet_angle,
                move_dir,
            },
            ..
        }) = entity.prev_move else {
            return None;
        };
    if move_dir == vec2::ZERO {
        // Cant continue after locked in place rotation
        return None;
    }
    if move_dir.y == 0 && !config.magnet.continue_horizontal {
        return None;
    }
    if prev_input != input && config.magnet.r#continue == ContinueConfig::Input {
        return None;
    }
    let new_pos = Position {
        cell: entity.pos.cell + magnet_angle.to_vec(),
        angle: entity.pos.angle.with_input(prev_input),
    };
    if is_blocked(state, new_pos.cell) {
        return None;
    }
    Some(EntityMove {
        entity_id: entity.id,
        used_input: prev_input,
        prev_pos: entity.pos,
        new_pos,
        move_type: EntityMoveType::MagnetContinue, // Can not continue magnet move more than 180 degrees
    })
}
