use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub cell: vec2<i32>,
    pub angle: IntAngle,
}

#[derive(Debug)]
pub struct Move {
    pub players: HashMap<usize, Position>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Input {
    Left,
    Skip,
    Right,
}

impl Input {
    pub fn delta(&self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Skip => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Effect {
    Jump,
}
impl Effect {
    fn apply(
        &self,
        state: &GameState,
        player_index: usize,
        input: Input,
        angle: IntAngle,
    ) -> Option<Position> {
        match self {
            Self::Jump => state.jump_from(player_index, input, angle),
        }
    }
}

pub struct Side {
    pub effect: Option<Effect>,
}

pub struct Player {
    pub pos: Position,
    pub sides: [Side; 4],
    pub mesh: Rc<ldtk::Mesh>, // TODO should not be here
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Nothing,
    Block,
    Disable,
}
impl Tile {
    pub fn is_blocking(&self) -> bool {
        use Tile::*;
        match self {
            Nothing => false,
            Block => true,
            Disable => true,
        }
    }
    pub fn is_trigger(&self) -> bool {
        use Tile::*;
        match self {
            Nothing => false,
            Block => true,
            Disable => false,
        }
    }
}

pub struct GameState {
    pub level: Rc<ldtk::Level>,
    pub tiles: HashMap<vec2<i32>, Tile>,
    pub players: Vec<Player>,
    pub selected_player: usize,
}
impl GameState {
    pub fn new(level: &Rc<ldtk::Level>) -> Self {
        let mut tiles = HashMap::new();
        for grid in level
            .layers
            .iter()
            .filter_map(|layer| layer.int_grid.as_ref())
        {
            for (&pos, value) in grid {
                tiles.insert(
                    pos,
                    match value.as_str() {
                        "block" => Tile::Block,
                        "disable" => Tile::Disable,
                        _ => unreachable!(),
                    },
                );
            }
        }
        Self {
            tiles,
            level: level.clone(),
            players: level
                .layers
                .iter()
                .flat_map(|layer| &layer.entities)
                .filter(|entity| entity.identifier == "Player")
                .map(|entity| Player {
                    mesh: entity.mesh.clone(),
                    sides: std::array::from_fn(|_| Side { effect: None }),
                    pos: Position {
                        cell: entity.pos,
                        angle: IntAngle::RIGHT,
                    },
                })
                .collect(),
            selected_player: 0,
        }
    }

    pub fn change_player_selection(&mut self, delta: isize) {
        let mut new = self.selected_player as isize + delta;
        if new < 0 {
            new = self.players.len() as isize - 1;
        } else if new >= self.players.len() as isize {
            new = 0;
        }
        self.selected_player = new.try_into().unwrap();
    }

    pub fn tile(&self, pos: vec2<i32>) -> Tile {
        self.tiles.get(&pos).copied().unwrap_or(Tile::Nothing)
    }

    pub fn is_blocked(&self, pos: vec2<i32>) -> bool {
        self.tile(pos).is_blocking() || self.players.iter().any(|player| player.pos.cell == pos)
    }

    pub fn is_trigger(&self, pos: vec2<i32>) -> bool {
        self.tile(pos).is_trigger() || self.players.iter().any(|player| player.pos.cell == pos)
    }

    pub fn perform_move(&mut self, r#move: &Move) {
        for (&player_index, &to) in &r#move.players {
            self.players[player_index].pos = to;
        }
    }

    fn gravity(&self, index: usize, _input: Input) -> Option<Position> {
        let mut pos = self.players[index].pos;
        pos.cell.y -= 1;
        if !self.is_blocked(pos.cell) {
            return Some(pos);
        }
        None
    }

    fn just_move(&self, index: usize, input: Input) -> Option<Position> {
        if input == Input::Skip {
            return None;
        }
        let mut new_pos = self.players[index].pos;
        let next_cell = new_pos.cell + vec2(input.delta(), 0);
        if !self.is_blocked(next_cell) {
            new_pos.cell = next_cell;
        }
        new_pos.angle = new_pos.angle.with_input(input);
        Some(new_pos)
    }

    fn jump_from(
        &self,
        player_index: usize,
        input: Input,
        jump_from: IntAngle,
    ) -> Option<Position> {
        log::debug!("Jumping from {jump_from:?}");
        let jump_to = jump_from.opposite();
        let pos = self.players[player_index].pos;
        let mut path = vec![vec2(0, 1), vec2(0, 2)];
        if jump_to.is_up() {
            path.push(vec2(input.delta(), 2));
        }
        let path = path
            .iter()
            .map(|&p| pos.cell + (jump_to - IntAngle::UP).rotate_vec(p));

        let mut new_pos = None;
        for p in path {
            if self.is_blocked(p) {
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
        new_pos
    }

    fn side_effects(&self, player_index: usize, input: Input) -> Option<Position> {
        let player = &self.players[player_index];
        for (side_index, side) in player.sides.iter().enumerate() {
            let side_angle = player.pos.angle + IntAngle::from_side(side_index);
            if self.is_trigger(player.pos.cell + side_angle.to_vec()) {
                if let Some(effect) = &side.effect {
                    if let Some(pos) = effect.apply(self, player_index, input, side_angle) {
                        return Some(pos);
                    }
                }
            }
        }
        None
    }

    fn move_player(&self, index: usize, input: Input) -> Option<Position> {
        let systems: &[&dyn Fn(&Self, usize, Input) -> Option<Position>] =
            &[&Self::side_effects, &Self::gravity, &Self::just_move];

        for system in systems {
            if let Some(pos) = system(self, index, input) {
                return Some(pos);
            }
        }
        None
    }

    pub fn check_move(&self, input: Input) -> Option<Move> {
        let mut result = Move {
            players: HashMap::new(),
        };
        for index in 0..self.players.len() {
            if let Some(new_pos) = self.move_player(
                index,
                if index == self.selected_player {
                    input
                } else {
                    Input::Skip
                },
            ) {
                result.players.insert(index, new_pos);
            }
        }
        result
            .players
            .retain(|&index, &mut to| self.players[index].pos != to);
        if result.players.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    pub fn selected_player(&self) -> &Player {
        &self.players[self.selected_player]
    }

    pub fn selected_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.selected_player]
    }
}
