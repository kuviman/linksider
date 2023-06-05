use super::*;

pub fn system(
    EntityMoveParams {
        state, entity_id, ..
    }: EntityMoveParams,
) -> Option<EntityMove> {
    if magnet::entity_magneted_angles(state, entity_id)
        .next()
        .is_some()
    {
        // No gravity when we have an active magnet
        return None;
    }
    if effects::entity_active_effects(state, entity_id)
        .any(|(_, effect)| matches!(effect.deref(), Effect::DisableGravity))
    {
        // Or any DisableGravity effect is active
        return None;
    }
    let entity = state.entities.get(&entity_id).unwrap();
    let mut new_pos = entity.pos;
    new_pos.cell.y -= 1;
    if !is_blocked(state, new_pos.cell) {
        return Some(EntityMove {
            entity_id: entity.id,
            used_input: Input::Skip,
            prev_pos: entity.pos,
            new_pos,
            move_type: EntityMoveType::Gravity,
        });
    }
    None
}
