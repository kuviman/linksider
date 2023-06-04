use super::*;

pub fn system(state: &GameState, entity_id: Id, _input: Input) -> Option<EntityMove> {
    let entity = state.entities.get(&entity_id).unwrap();
    // TODO not only players?
    if !entity.properties.player {
        return None;
    }
    if let Some(goal) = state
        .goals
        .iter()
        .find(|goal| goal.pos.normalize() == entity.pos.normalize())
    {
        return Some(EntityMove {
            entity_id: entity.id,
            used_input: Input::Skip,
            prev_pos: entity.pos,
            new_pos: entity.pos,
            move_type: EntityMoveType::EnterGoal { goal_id: goal.id },
        });
    }

    None
}

impl GameState {
    pub fn finished(&self) -> bool {
        self.goals.is_empty()
    }
}
