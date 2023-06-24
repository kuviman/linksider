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
    let mut cells_traveled = 0;
    let mut blocked_angle = None;
    let mut prev_cell = entity.pos.cell;
    for p in path {
        if is_blocked(state, p) {
            blocked_angle = Some(IntAngle::from_vec(p - prev_cell));
            break;
        }
        prev_cell = p;
        new_pos = Some(Position {
            cell: p,
            angle: if jump_to.is_up() {
                pos.angle + input
            } else {
                pos.angle
            },
        });
        cells_traveled += 1;
    }
    if let Some(new_pos) = new_pos {
        Some(EntityMove {
            entity_id: entity.id,
            used_input: input,
            prev_pos: entity.pos,
            new_pos,
            move_type: EntityMoveType::Jump {
                from: jump_from,
                blocked_angle,
                cells_traveled,
                jump_force: 3,
            },
        })
    } else {
        None
    }
}
