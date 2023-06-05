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

impl Tile {
    pub fn is_blocking(&self) -> bool {
        use Tile::*;
        match self {
            Nothing => false,
            Block => true,
            Disable => true,
            Cloud => false,
        }
    }
}

pub fn is_blocked(state: &GameState, pos: vec2<i32>) -> bool {
    state.tile(pos).is_blocking() || state.entities.iter().any(|entity| entity.pos.cell == pos)
}

fn check_entity_move(
    state: &GameState,
    entity_id: Id,
    input: Input,
) -> Option<Collection<EntityMove>> {
    macro_rules! system {
        ($f:expr) => {
            if let Some(moves) = $f(state, entity_id, input) {
                return Some(moves);
            }
        };
    }

    fn simple(
        f: impl Fn(&GameState, Id, Input) -> Option<EntityMove>,
    ) -> impl Fn(&GameState, Id, Input) -> Option<Collection<EntityMove>> {
        move |state, entity_id, input| {
            f(state, entity_id, input).map(|entity_move| {
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

fn check_moves(state: &GameState, input: Input) -> Collection<EntityMove> {
    let mut result = Collection::new();
    for &id in state.entities.ids() {
        if let Some(moves) = check_entity_move(
            state,
            id,
            if Some(id) == state.selected_player {
                input
            } else {
                Input::Skip
            },
        ) {
            // TODO check for conflicts
            result.extend(moves);
        }
    }
    result
}

fn perform_moves(state: &mut GameState, moves: &Collection<EntityMove>) {
    for entity_move in moves {
        let entity = state.entities.get_mut(&entity_move.entity_id).unwrap();
        assert_eq!(entity.pos, entity_move.prev_pos);
        entity.pos = entity_move.new_pos;
        if let EntityMoveType::EnterGoal { goal_id } = entity_move.move_type {
            state.goals.remove(&goal_id);
            state.entities.remove(&entity_move.entity_id);
        }
    }
}

impl GameState {
    pub fn process_turn(&mut self, input: Input) -> Option<Moves> {
        let state = self;
        state.stable = false;
        let result = Moves {
            entity_moves: {
                let moves = check_moves(state, input);
                // TODO check for conflicts
                for entity in state.entities.iter_mut() {
                    entity.prev_pos = entity.pos;
                    entity.prev_move = moves.get(&entity.id).cloned();
                }
                perform_moves(state, &moves);
                moves
            },
            collected_powerups: powerups::process(state),
        };
        if result.collected_powerups.is_empty() && result.entity_moves.is_empty() {
            state.stable = true;
            return None;
        }
        Some(result)
    }
}
