use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub cell: vec2<i32>,
    pub angle: IntAngle,
}

impl Position {
    fn from_ldtk_entity(entity: &ldtk::Entity, down_angle: IntAngle) -> Self {
        Self {
            cell: entity.pos,
            angle: entity.fields.get("Side").map_or(IntAngle::DOWN, |value| {
                match value.as_str().expect("Side value not a string WTF") {
                    "Down" => IntAngle::DOWN,
                    "Right" => IntAngle::RIGHT,
                    "Left" => IntAngle::LEFT,
                    "Up" => IntAngle::UP,
                    _ => unreachable!("Unexpected side value {value:?}"),
                }
            }) - IntAngle::DOWN
                + down_angle,
        }
    }
    pub fn normalize(&self) -> Self {
        Self {
            cell: self.cell,
            angle: self.angle.normalize(),
        }
    }
}

#[derive(Debug)]
pub struct Moves {
    pub players: HashMap<usize, PlayerMove>,
}

#[derive(Debug, Clone)]
pub enum PlayerMoveType {
    Magnet {
        magnet_angle: IntAngle,
        move_dir: vec2<i32>,
    },
    Unsorted, // TODO remove
}

#[derive(Debug, Clone)]
pub struct PlayerMove {
    pub used_input: Input,
    pub new_pos: Position,
    pub move_type: PlayerMoveType,
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
    Slide,
    Magnet,
}

impl Effect {
    pub fn activate_self(&self) -> bool {
        match self {
            Effect::Jump => true,
            Effect::Slide => true,
            Effect::Magnet => true,
        }
    }
    pub fn activate_other(&self) -> bool {
        match self {
            Effect::Jump => true,
            Effect::Slide => true,
            Effect::Magnet => false,
        }
    }
    fn apply(
        &self,
        state: &GameState,
        player_index: usize,
        input: Input,
        angle: IntAngle,
    ) -> Option<PlayerMove> {
        match self {
            Self::Jump => state.jump_from(player_index, input, angle),
            Self::Slide => state.slide(player_index, input, angle),
            Self::Magnet => {
                // Magnets are affecting gravity of regular move
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct Side {
    pub effect: Option<Effect>,
}

pub struct Player {
    pub prev_pos: Position,
    pub prev_move: Option<PlayerMove>,
    pub pos: Position,
    pub sides: [Side; 4],
    pub mesh: Rc<ldtk::Mesh>, // TODO should not be here
}

impl Player {
    // TODO better name?
    pub fn maybe_override_input(&self, input: Input) -> Input {
        let last_move = self.pos.cell - self.prev_pos.cell;
        if last_move.x != 0 && last_move.y == 0 {
            Input::from_sign(last_move.x)
        } else {
            input
        }
    }

    /// Side index by absolute side angle
    pub fn side_index(&self, angle: IntAngle) -> usize {
        (angle - self.side_angle(0)).normalize().to_i32() as usize
    }

    /// Absolute side angle
    pub fn side_angle(&self, side_index: usize) -> IntAngle {
        // Side 0 is right, side 1 is up, etc
        // (if player is not rotated)
        Self::relative_side_angle(side_index) + self.pos.angle
    }

    pub fn relative_side_angle(side_index: usize) -> IntAngle {
        IntAngle::from_i32(side_index as i32)
    }
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
    Cloud,
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
    pub fn is_trigger(&self) -> bool {
        use Tile::*;
        match self {
            Nothing => false,
            Block => true,
            Disable => false,
            Cloud => true,
        }
    }
}

pub struct GameState {
    pub level: Rc<ldtk::Level>, // TODO remove, this is not state
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
                        "cloud" => Tile::Cloud,
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
                .map(|entity| {
                    let pos = Position::from_ldtk_entity(entity, IntAngle::RIGHT);
                    Player {
                        mesh: entity.mesh.clone(),
                        sides: std::array::from_fn(|_| Side { effect: None }),
                        pos,
                        prev_pos: pos,
                        prev_move: None,
                    }
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
                            "Magnet" => Effect::Magnet,
                            "Slide" => Effect::Slide,
                            _ => unimplemented!("{name:?} power is unimplemented"),
                        },
                        pos: Position::from_ldtk_entity(entity, IntAngle::DOWN),
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
        for (&player_index, player_move) in players {
            self.players[player_index].pos = player_move.new_pos;
        }
    }

    fn gravity(&self, player_index: usize, _input: Input) -> Option<PlayerMove> {
        if self.player_magneted_angles(player_index).next().is_some() {
            // No gravity when we have an active magnet
            return None;
        }
        let mut new_pos = self.players[player_index].pos;
        new_pos.cell.y -= 1;
        if !self.is_blocked(new_pos.cell) {
            return Some(PlayerMove {
                used_input: Input::Skip,
                new_pos,
                move_type: PlayerMoveType::Unsorted,
            });
        }
        None
    }

    fn player_active_effects(
        &self,
        player_index: usize,
    ) -> impl Iterator<Item = (IntAngle, &Effect)> + '_ {
        let player = &self.players[player_index];
        let mut result = vec![];
        for (side_index, side) in player.sides.iter().enumerate() {
            let side_angle = player.side_angle(side_index);
            let side_cell = player.pos.cell + side_angle.to_vec();
            if self.is_trigger(side_cell) {
                if let Some(effect) = &side.effect {
                    if effect.activate_self() {
                        result.push((side_angle, effect));
                    }
                }
            }
            for other_player in &self.players {
                if other_player.pos.cell == side_cell {
                    let other_side_index = other_player.side_index(side_angle.opposite());
                    let other_side = &other_player.sides[other_side_index];
                    if let Some(effect) = &other_side.effect {
                        if effect.activate_other() {
                            result.push((side_angle, effect));
                        }
                    }
                }
            }
        }
        result.into_iter()
    }

    fn player_magneted_angles(&self, player_index: usize) -> impl Iterator<Item = IntAngle> + '_ {
        self.player_active_effects(player_index)
            .flat_map(|(side, effect)| {
                if let Effect::Magnet = effect {
                    Some(side)
                } else {
                    None
                }
            })
    }

    fn just_move(&self, player_index: usize, input: Input) -> Option<PlayerMove> {
        if input == Input::Skip {
            return None;
        }
        let player = &self.players[player_index];

        let magneted_angles: HashSet<IntAngle> = self
            .player_magneted_angles(player_index)
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
        let find_magnet_direction = |f: &dyn Fn(IntAngle) -> IntAngle| {
            let mut possible = magneted_angles
                .iter()
                .map(|&angle| (angle, f(angle).normalize()))
                .filter(|(_, dir)| {
                    !magneted_angles.contains(dir)
                        && !self.is_blocked(player.pos.cell + dir.to_vec())
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

        let mut new_pos = player.pos;
        let next_cell = new_pos.cell + direction.move_dir;
        if !self.is_blocked(next_cell) {
            new_pos.cell = next_cell;
        }
        new_pos.angle = new_pos.angle.with_input(input);
        Some(PlayerMove {
            used_input: input,
            new_pos,
            move_type: if let Some(magnet_angle) = direction.magnet_angle {
                PlayerMoveType::Magnet {
                    magnet_angle,
                    move_dir: direction.move_dir,
                }
            } else {
                PlayerMoveType::Unsorted
            },
        })
    }

    fn slide(&self, player_index: usize, input: Input, side: IntAngle) -> Option<PlayerMove> {
        if !side.is_down() {
            return None;
        }
        log::debug!("Sliding on {side:?}");

        let player = &self.players[player_index];
        let input = player.maybe_override_input(input);

        let new_pos = Position {
            cell: player.pos.cell + vec2(input.delta(), 0),
            angle: player.pos.angle,
        };
        if self.is_blocked(new_pos.cell) {
            return None;
        }
        Some(PlayerMove {
            used_input: input,
            new_pos,
            move_type: PlayerMoveType::Unsorted,
        })
    }

    fn jump_from(
        &self,
        player_index: usize,
        input: Input,
        jump_from: IntAngle,
    ) -> Option<PlayerMove> {
        log::debug!("Jumping from {jump_from:?}");

        let player = &self.players[player_index];
        let input = player.maybe_override_input(input);

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
        if let Some(new_pos) = new_pos {
            Some(PlayerMove {
                used_input: input,
                new_pos,
                move_type: PlayerMoveType::Unsorted,
            })
        } else {
            None
        }
    }

    fn side_effects(&self, player_index: usize, input: Input) -> Option<PlayerMove> {
        for (side, effect) in self.player_active_effects(player_index) {
            if let Some(pos) = effect.apply(self, player_index, input, side) {
                return Some(pos);
            }
        }
        None
    }

    fn continue_magnet_move(&self, index: usize, input: Input) -> Option<PlayerMove> {
        let player = &self.players[index];
        let Some(PlayerMove {
            used_input: prev_input,
            move_type: PlayerMoveType::Magnet {
                magnet_angle,
                move_dir,
            },
            ..
        }) = player.prev_move else {
            return None;
        };
        if move_dir == vec2::ZERO {
            // Cant continue after locked in place rotation
            return None;
        }
        if prev_input != input {
            return None;
        }
        let new_pos = Position {
            cell: player.pos.cell + magnet_angle.to_vec(),
            angle: player.pos.angle.with_input(input),
        };
        if self.is_blocked(new_pos.cell) {
            return None;
        }
        Some(PlayerMove {
            used_input: input,
            new_pos,
            move_type: PlayerMoveType::Unsorted, // Can not continue magnet move more than 180
                                                 // degrees
        })
    }

    fn check_player_move(&self, index: usize, input: Input) -> Option<PlayerMove> {
        let systems: &[&dyn Fn(&Self, usize, Input) -> Option<PlayerMove>] = &[
            &Self::continue_magnet_move,
            &Self::side_effects,
            &Self::gravity,
            &Self::just_move,
        ];

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
            if let Some(player_move) = self.check_player_move(
                index,
                if index == self.selected_player {
                    input
                } else {
                    Input::Skip
                },
            ) {
                result.players.insert(index, player_move);
            }
        }
        if result.players.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    // TODO not return anything
    pub fn process_turn(&mut self, input: Input) -> Option<Moves> {
        self.process_powerups();
        let result = self.check_moves(input);
        for (player_index, player) in self.players.iter_mut().enumerate() {
            player.prev_pos = player.pos;
            player.prev_move = result
                .as_ref()
                .and_then(|moves| moves.players.get(&player_index))
                .cloned();
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
                let player_side = player.side_index(powerup.pos.angle);
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
