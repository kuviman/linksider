use super::*;

pub fn system(state: &GameState, entity_id: Id, input: Input) -> Option<Collection<EntityMove>> {
    if input == Input::Skip {
        return None;
    }
    let entity = state.entities.get(&entity_id).unwrap();

    let magneted_angles: HashSet<IntAngle> = magnet::entity_magneted_angles(state, entity_id)
        .map(|angle| angle.normalize())
        .collect();

    struct Direction {
        magnet_angle: Option<IntAngle>,
        move_dir: vec2<i32>,
    }

    let mut left = Direction {
        magnet_angle: None,
        move_dir: vec2(-1, 0),
    };
    let mut right = Direction {
        magnet_angle: None,
        move_dir: vec2(1, 0),
    };

    // Can only move normally if we have ground below us
    if !is_blocked(state, entity.pos.cell + vec2(0, -1)) {
        left.move_dir = vec2::ZERO;
        right.move_dir = vec2::ZERO;
    }

    let find_magnet_direction = |f: &dyn Fn(IntAngle) -> IntAngle| {
        let mut possible = magneted_angles
            .iter()
            .map(|&angle| (angle, f(angle).normalize()))
            .filter(|(_, dir)| {
                !magneted_angles.contains(dir) && !is_blocked(state, entity.pos.cell + dir.to_vec())
            })
            .map(|(magnet_angle, dir)| Direction {
                magnet_angle: Some(magnet_angle),
                move_dir: dir.to_vec(),
            });
        let result = possible.next();
        if result.is_some() && possible.next().is_some() {
            // Means we are mageneted on opposite sides so we are stuck
            return None;
        }
        result
    };
    if let Some(magneted) = find_magnet_direction(&IntAngle::rotate_clockwise) {
        left = magneted;
    }
    if let Some(magneted) = find_magnet_direction(&IntAngle::rotate_counter_clockwise) {
        right = magneted;
    };

    let locked = magneted_angles
        .iter()
        .any(|angle| magneted_angles.contains(&angle.opposite()));
    if locked {
        left.move_dir = vec2::ZERO;
        right.move_dir = vec2::ZERO;
    }

    let direction = match input {
        Input::Left => left,
        Input::Right => right,
        Input::Skip => unreachable!(),
    };

    let mut new_pos = entity.pos;
    let next_cell = new_pos.cell + direction.move_dir;
    let mut result = Collection::new();
    if !is_blocked(state, next_cell) {
        new_pos.cell = next_cell;
    }
    if let Some(next_entity) = state
        .entities
        .iter()
        .find(|entity| entity.pos.cell == next_cell)
    {
        if next_entity.properties.pushable {
            let next_next_cell = next_cell + direction.move_dir;
            if !is_blocked(state, next_next_cell) {
                new_pos.cell = next_cell;
                result.insert(EntityMove {
                    entity_id: next_entity.id,
                    used_input: Input::Skip,
                    prev_pos: next_entity.pos,
                    new_pos: Position {
                        cell: next_next_cell,
                        angle: next_entity
                            .pos
                            .angle
                            .with_input(Input::from_sign(direction.move_dir.x)),
                    },
                    move_type: EntityMoveType::Pushed,
                });
            }
        }
    }
    new_pos.angle = new_pos.angle.with_input(input);
    result.insert(EntityMove {
        entity_id: entity.id,
        used_input: input,
        prev_pos: entity.pos,
        new_pos,
        move_type: if let Some(magnet_angle) = direction.magnet_angle {
            EntityMoveType::Magnet {
                magnet_angle,
                move_dir: direction.move_dir,
            }
        } else {
            EntityMoveType::Move
        },
    });
    Some(result)
}
