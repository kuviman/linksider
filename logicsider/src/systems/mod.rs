use super::*;

pub mod effects;
pub mod goal;
pub mod gravity;
pub mod jump;
pub mod just_move;
pub mod magnet;
pub mod powerups;
pub mod slide;

impl Entity {
    // TODO better name?
    pub fn maybe_override_input(&self, input: Input) -> Input {
        let last_move = self.pos.cell - self.prev_pos.cell;
        if last_move.x != 0 && last_move.y == 0 {
            Input::from_sign(last_move.x)
        } else if last_move == vec2::ZERO && self.pos.angle != self.prev_pos.angle {
            Input::from_sign(-(self.pos.angle - self.prev_pos.angle).to_i32())
        } else {
            input
        }
    }
}

pub fn is_blocked(state: &GameState, pos: vec2<i32>) -> bool {
    if state.reserved_cells.contains(&pos) {
        return true;
    }
    state
        .entities
        .iter()
        .any(|entity| entity.pos.cell == pos && entity.properties.block)
}

#[derive(Copy, Clone)]
pub struct EntityMoveParams<'a> {
    state: &'a GameState,
    config: &'a Config,
    entity_id: Id,
    input: Input,
}

fn check_entity_move(params: EntityMoveParams) -> Option<Collection<EntityMove>> {
    macro_rules! system {
        ($f:expr) => {
            if let Some(moves) = $f(params) {
                return Some(moves);
            }
        };
    }

    fn simple(
        f: impl Fn(EntityMoveParams) -> Option<EntityMove>,
    ) -> impl Fn(EntityMoveParams) -> Option<Collection<EntityMove>> {
        move |params| {
            f(params).map(|entity_move| {
                let mut result = Collection::new();
                result.insert(entity_move);
                result
            })
        }
    }

    system!(simple(magnet::continue_move));
    system!(simple(effects::side_effects));
    system!(simple(gravity::system));
    system!(simple(goal::system));
    system!(just_move::system);

    None
}

impl GameState {
    fn try_start_moves(
        &mut self,
        config: &Config,
        inputs: &HashMap<Id, Input>,
        event_handler: &mut impl FnMut(Event),
    ) {
        powerups::process(self, event_handler);

        self.reserved_cells.clear();
        for entity_move in &self.moves {
            self.reserved_cells
                .extend(entity_move.cells_reserved.iter().copied());
        }

        let mut entity_moves = Vec::new();
        let stable_entity_ids: Vec<Id> = self.stable_entities().map(|entity| entity.id).collect();
        for entity_id in stable_entity_ids {
            let entity = self.entities.get(&entity_id).unwrap();
            if entity.properties.r#static {
                continue;
            }
            if let Some(moves) = check_entity_move(EntityMoveParams {
                state: self,
                config,
                entity_id: entity.id,
                input: inputs.get(&entity.id).copied().unwrap_or(Input::Skip),
            }) {
                self.reserved_cells.extend(
                    moves
                        .iter()
                        .flat_map(|entity_move| entity_move.cells_reserved.iter().copied()),
                );
                entity_moves.push(moves);
            }
        }

        for entity in &mut self.entities {
            entity.prev_pos = entity.pos;
            entity.prev_move = None;
        }

        for entity_moves in entity_moves {
            // if entity_moves.iter().any(|entity_move| {
            //     entity_move
            //         .cells_reserved
            //         .iter()
            //         .any(|cell| self.reserved_cells.contains(cell))
            // }) {
            //     continue;
            // }
            for entity_move in &entity_moves {
                // self.reserved_cells
                //     .extend(entity_move.cells_reserved.iter().copied());
                let prev = self
                    .entities
                    .get_mut(&entity_move.entity_id)
                    .unwrap()
                    .current_move
                    .replace(entity_move.clone());
                assert!(prev.is_none());
                event_handler(Event::MoveStarted(entity_move.clone()));
            }
            self.moves.extend(entity_moves);
        }
    }

    fn end_move(&mut self, config: &Config, entity_move: EntityMove) {
        let entity = self.entities.get_mut(&entity_move.entity_id).unwrap();
        assert!(!entity.properties.r#static);
        assert_eq!(entity.pos, entity_move.prev_pos);
        entity.prev_pos = entity.pos;
        entity.prev_move = Some(entity_move.clone());
        entity.pos = entity_move.new_pos;
        entity.current_move = None;
        if let EntityMoveType::EnterGoal { goal_id } = entity_move.move_type {
            self.goals.remove(&goal_id);
            self.entities.remove(&entity_move.entity_id);
        }
    }

    pub fn update(
        &mut self,
        config: &Config,
        inputs: &HashMap<Id, Input>,
        delta_time: Time,
        mut simulation_step_handler: impl FnMut(&Self),
        mut event_handler: impl FnMut(Event),
    ) {
        let target_time = self.current_time + delta_time;
        while self.current_time < target_time {
            // Ending moves that have ended
            while let Some(earliest_ending_move) = self.moves.peek() {
                if earliest_ending_move.end_time <= self.current_time {
                    assert!(earliest_ending_move.end_time == self.current_time);
                    let entity_move = self.moves.pop().unwrap();
                    event_handler(Event::MoveEnded(entity_move.clone()));
                    self.end_move(config, entity_move);
                } else {
                    break;
                }
            }
            self.try_start_moves(config, inputs, &mut event_handler);
            self.stable = self.moves.is_empty();
            if let Some(earliest_ending_move) = self.moves.peek() {
                if earliest_ending_move.end_time <= target_time {
                    self.current_time = earliest_ending_move.end_time;
                    simulation_step_handler(self);
                    continue;
                }
            }
            // Nothing happened
            self.current_time = target_time;
        }
        assert!(self.current_time == target_time);
    }
}
