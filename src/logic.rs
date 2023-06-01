use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub cell: vec2<i32>,
    pub angle: IntAngle,
}

impl Position {
    pub fn normalize(&self) -> Self {
        Self {
            cell: self.cell,
            angle: self.angle.normalize(),
        }
    }
}

#[derive(Debug)]
pub struct Moves {
    pub players: HashMap<usize, Position>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Input {
    Left,
    Skip,
    Right,
}

impl Input {
    pub fn from_sign(x: i32) -> Self {
        match x.signum() {
            -1 => Self::Left,
            0 => Self::Skip,
            1 => Self::Right,
            _ => unreachable!(),
        }
    }
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
    pub prev_pos: Position,
    pub pos: Position,
    pub sides: [Side; 4],
    pub mesh: Rc<ldtk::Mesh>, // TODO should not be here
}

pub struct Goal {
    pub pos: Position,
    pub mesh: Rc<ldtk::Mesh>, // TODO should not be here
}

pub struct Powerup {
    pub pos: Position,
    pub effect: Effect,
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
    pub powerups: Vec<Powerup>,
    pub selected_player: usize,
    pub goals: Vec<Goal>,
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
                    prev_pos: Position {
                        cell: entity.pos,
                        angle: IntAngle::RIGHT,
                    },
                })
                .collect(),
            powerups: level
                .layers
                .iter()
                .flat_map(|layer| &layer.entities)
                .filter_map(|entity| {
                    entity.identifier.strip_suffix("Power").map(|name| Powerup {
                        effect: match name {
                            "Jump" => Effect::Jump,
                            _ => unimplemented!("{name:?} power is unimplemented"),
                        },
                        pos: Position {
                            cell: entity.pos,
                            angle: IntAngle::DOWN,
                        },
                        mesh: entity.mesh.clone(),
                    })
                })
                .collect(),
            goals: level
                .layers
                .iter()
                .flat_map(|layer| &layer.entities)
                .filter(|entity| entity.identifier == "Goal")
                .map(|entity| Goal {
                    pos: Position {
                        cell: entity.pos,
                        angle: IntAngle::RIGHT,
                    },
                    mesh: entity.mesh.clone(),
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

    pub fn perform_moves(&mut self, moves: &Moves) {
        let Moves { players } = moves;
        for (&player_index, &to) in players {
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

        let player = &self.players[player_index];

        let input = {
            let last_move = player.pos.cell - player.prev_pos.cell;
            if last_move.x.abs() == 1 && last_move.y == 0 {
                Input::from_sign(last_move.x)
            } else {
                input
            }
        };

        let jump_to = jump_from.opposite();
        let pos = player.pos;
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

    fn check_player_move(&self, index: usize, input: Input) -> Option<Position> {
        let systems: &[&dyn Fn(&Self, usize, Input) -> Option<Position>] =
            &[&Self::side_effects, &Self::gravity, &Self::just_move];

        for system in systems {
            if let Some(pos) = system(self, index, input) {
                return Some(pos);
            }
        }
        None
    }

    fn check_moves(&self, input: Input) -> Option<Moves> {
        let mut result = Moves {
            players: HashMap::new(),
        };
        for index in 0..self.players.len() {
            if let Some(new_pos) = self.check_player_move(
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

    pub fn process_turn(&mut self, input: Input) -> Option<Moves> {
        self.process_powerups();
        let result = self.check_moves(input);
        for player in &mut self.players {
            player.prev_pos = player.pos;
        }
        result
    }

    fn process_powerups(&mut self) {
        #[derive(Debug)]
        pub struct CollectedPowerup {
            pub player: usize,
            pub player_side: usize,
            pub powerup: usize,
        }

        let mut collected = Vec::new();
        for (player_index, player) in self.players.iter().enumerate() {
            for (powerup_index, powerup) in self.powerups.iter().enumerate() {
                if player.pos.cell != powerup.pos.cell {
                    continue;
                }
                let player_side = (powerup.pos.angle - player.pos.angle).side_index();
                if player.sides[player_side].effect.is_none() {
                    collected.push(CollectedPowerup {
                        player: player_index,
                        player_side: player_side,
                        powerup: powerup_index,
                    })
                }
            }
        }
        for event in collected {
            let powerup = self.powerups.remove(event.powerup);
            let prev_effect = self.players[event.player].sides[event.player_side]
                .effect
                .replace(powerup.effect);
            assert!(prev_effect.is_none());
        }
    }

    pub fn selected_player(&self) -> &Player {
        &self.players[self.selected_player]
    }

    pub fn selected_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.selected_player]
    }

    pub fn finished(&self) -> bool {
        // TODO remove goal on player touch, have goals.is_empty() here
        self.goals.iter().all(|goal| {
            self.players
                .iter()
                .any(|player| player.pos.normalize() == goal.pos.normalize())
        })
    }
}
