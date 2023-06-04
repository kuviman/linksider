use super::*;

pub fn system(
    state: &GameState,
    entity_id: Id,
    input: Input,
    jump_from: IntAngle,
) -> Option<EntityMove> {
    log::debug!("Jumping from {jump_from:?}");

    let entity = state.entities.get(&entity_id).unwrap();
    let input = entity.maybe_override_input(input);

    let jump_to = jump_from.opposite();
    let pos = entity.pos;
    let mut path = vec![vec2(0, 1), vec2(0, 2)];
    if jump_to.is_up() {
        path.push(vec2(input.delta(), 2));
    }
    let path = path
        .iter()
        .map(|&p| pos.cell + (jump_to - IntAngle::UP).rotate_vec(p));

    let mut new_pos = None;
    for p in path {
        if is_blocked(state, p) {
            break;
        }
        new_pos = Some(Position {
            cell: p,
            angle: if jump_to.is_up() {
                pos.angle + input
            } else {
                pos.angle
            },
        });
    }
    if let Some(new_pos) = new_pos {
        Some(EntityMove {
            entity_id: entity.id,
            used_input: input,
            prev_pos: entity.pos,
            new_pos,
            move_type: EntityMoveType::Jump,
        })
    } else {
        None
    }
}
