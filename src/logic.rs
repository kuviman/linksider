use super::*;

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub struct Rotation(i32);

impl Rotation {
    pub const ZERO: Self = Self(0);

    pub fn to_matrix(&self) -> mat3<f32> {
        mat3::rotate(self.0 as f32 * f32::PI / 2.0)
    }

    pub fn with_input(self, input: Input) -> Self {
        match input {
            Input::Left => self.rotate_counter_clockwise(),
            Input::Skip => self,
            Input::Right => self.rotate_clockwise(),
        }
    }

    pub fn rotate_counter_clockwise(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn rotate_clockwise(self) -> Self {
        Self(self.0 - 1)
    }
}

impl Debug for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0.rem_euclid(4) {
            0 => write!(f, "Down"),
            1 => write!(f, "Right"),
            2 => write!(f, "Up"),
            3 => write!(f, "Left"),
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub cell: vec2<i32>,
    pub rot: Rotation,
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

pub struct Player {
    pub pos: Position,
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
                    pos: Position {
                        cell: entity.pos,
                        rot: Rotation::ZERO,
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
        let next_cell = new_pos.cell
            + match input {
                Input::Left => vec2(-1, 0),
                Input::Skip => vec2(0, 0),
                Input::Right => vec2(1, 0),
            };
        if !self.is_blocked(next_cell) {
            new_pos.cell = next_cell;
        }
        new_pos.rot = new_pos.rot.with_input(input);
        Some(new_pos)
    }

    fn move_player(&self, index: usize, input: Input) -> Option<Position> {
        let systems: &[&dyn Fn(&Self, usize, Input) -> Option<Position>] =
            &[&Self::gravity, &Self::just_move];

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
}
