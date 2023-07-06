use super::*;

pub fn system(
    state: &GameState,
    entity_id: Id,
    input: Input,
    side: IntAngle,
) -> Option<EntityMove> {
    if !side.is_down() {
        return None;
    }
    log::debug!("Sliding on {side:?}");

    let entity = state.entities.get(&entity_id).unwrap();

    let slide_with_input = |input: Input| -> Option<EntityMove> {
        let prev_pos = entity.pos;
        let new_pos = Position {
            cell: entity.pos.cell + vec2(input.delta(), 0),
            angle: entity.pos.angle,
        };
        if is_blocked(state, new_pos.cell) {
            return None;
        }
        Some(EntityMove {
            entity_id: entity.id,
            used_input: input,
            prev_pos,
            new_pos,
            move_type: if let Some(EntityMove {
                move_type: EntityMoveType::SlideStart | EntityMoveType::SlideContinue,
                ..
            }) = &entity.prev_move
            {
                EntityMoveType::SlideContinue
            } else {
                EntityMoveType::SlideStart
            },
            start_time: state.current_time,
            end_time: state.current_time + Time::ONE,
            cells_reserved: [prev_pos.cell, new_pos.cell].into_iter().collect(),
        })
    };
    slide_with_input(entity.maybe_override_input(input)).or(slide_with_input(input))
}
