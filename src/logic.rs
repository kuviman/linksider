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
    pub entities: HashMap<usize, EntityMove>,
}

#[derive(Debug, Clone)]
pub enum EntityMoveType {
    Magnet {
        magnet_angle: IntAngle,
        move_dir: vec2<i32>,
    },
    Unsorted, // TODO remove
}

#[derive(Debug, Clone)]
pub struct EntityMove {
    pub used_input: Input,
    pub new_pos: Position,
    pub move_type: EntityMoveType,
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
    DisableGravity,
    DisableTrigger,
}

impl Effect {
    // TODO derive
    pub fn from_str(name: &str) -> Self {
        match name {
            "Jump" => Self::Jump,
            "Magnet" => Self::Magnet,
            "Slide" => Self::Slide,
            "DisableGravity" => Self::DisableGravity,
            "DisableTrigger" => Self::DisableTrigger,
            _ => unimplemented!("{name:?} effect is unimplemented"),
        }
    }
    /// Whether having this effect enables or disables other effects on the touching side
    pub fn is_trigger(&self) -> bool {
        match self {
            Self::DisableTrigger => false,
            _ => true,
        }
    }
    pub fn activate_self(&self) -> bool {
        true
    }
    pub fn activate_other(&self) -> Option<Self> {
        match self {
            Self::Jump | Self::Slide => Some(self.clone()),
            Self::Magnet => Some(Self::DisableGravity),
            Self::DisableGravity | Self::DisableTrigger => None,
        }
    }
    fn apply(
        &self,
        state: &GameState,
        entity_index: usize,
        input: Input,
        angle: IntAngle,
    ) -> Option<EntityMove> {
        match self {
            Self::Jump => state.jump_from(entity_index, input, angle),
            Self::Slide => state.slide(entity_index, input, angle),
            // Some effects are handled in other systems
            Self::Magnet | Self::DisableTrigger | Self::DisableGravity => None,
        }
    }
}

#[derive(Debug)]
pub struct Side {
    pub effect: Option<Effect>,
}

pub struct Properties {
    pub block: bool,
    pub trigger: bool,
    pub player: bool,
}

/// Box entity
pub struct Entity {
    pub properties: Properties,
    pub pos: Position,
    pub prev_pos: Position,
    pub prev_move: Option<EntityMove>,
    pub sides: [Side; 4],
    pub mesh: Rc<ldtk::Mesh>, // TODO should not be here
}

impl Entity {
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
        // (if entity is not rotated)
        Self::relative_side_angle(side_index) + self.pos.angle
    }

    pub fn side_at_angle(&self, angle: IntAngle) -> &Side {
        &self.sides[self.side_index(angle)]
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
    pub entities: Vec<Entity>,
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
        let mut result = Self {
            tiles,
            level: level.clone(),
            entities: default(),
            powerups: default(),
            goals: default(),
            selected_player: 0,
        };
        for entity in level.layers.iter().flat_map(|layer| &layer.entities) {
            let pos = Position::from_ldtk_entity(
                entity,
                if entity.identifier.starts_with("Power") {
                    IntAngle::DOWN
                } else {
                    IntAngle::RIGHT
                },
            );
            if let Some(effect) = entity.identifier.strip_suffix("Power") {
                result.powerups.push(Powerup {
                    effect: Effect::from_str(effect),
                    pos: Position::from_ldtk_entity(entity, IntAngle::DOWN),
                    mesh: entity.mesh.clone(),
                });
            } else {
                match entity.identifier.as_str() {
                    "Goal" => {
                        result.goals.push(Goal {
                            pos: Position {
                                cell: entity.pos,
                                angle: IntAngle::RIGHT,
                            },
                            mesh: entity.mesh.clone(),
                        });
                    }
                    entity_name => {
                        let properties = match entity_name {
                            "Player" => Properties {
                                block: true,
                                trigger: true,
                                player: true,
                            },
                            "Crate" => Properties {
                                block: true,
                                trigger: true,
                                player: false,
                            },
                            _ => unimplemented!("Entity {entity_name:?} unimplemented"),
                        };
                        result.entities.push(Entity {
                            properties,
                            mesh: entity.mesh.clone(),
                            sides: std::array::from_fn(|_| Side { effect: None }),
                            pos,
                            prev_pos: pos,
                            prev_move: None,
                        });
                    }
                }
            }
        }
        result
    }

    pub fn change_player_selection(&mut self, delta: isize) {
        let player_indices: Vec<usize> = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                if entity.properties.player {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();
        let index_index = player_indices
            .iter()
            .position(|&index| index == self.selected_player)
            .unwrap();
        let mut new = index_index as isize + delta;
        if new < 0 {
            new = player_indices.len() as isize - 1;
        } else if new >= player_indices.len() as isize {
            new = 0;
        }
        self.selected_player = player_indices[usize::try_from(new).unwrap()];
    }

    pub fn tile(&self, pos: vec2<i32>) -> Tile {
        self.tiles.get(&pos).copied().unwrap_or(Tile::Nothing)
    }

    pub fn is_blocked(&self, pos: vec2<i32>) -> bool {
        self.tile(pos).is_blocking() || self.entities.iter().any(|entity| entity.pos.cell == pos)
    }

    pub fn is_trigger(&self, pos: vec2<i32>, angle: IntAngle) -> bool {
        self.tile(pos).is_trigger()
            || self.entities.iter().any(|entity| {
                entity.pos.cell == pos
                    && entity
                        .side_at_angle(angle)
                        .effect
                        .as_ref()
                        .map_or(true, Effect::is_trigger)
            })
    }

    pub fn perform_moves(&mut self, moves: &Moves) {
        let Moves { entities } = moves;
        for (&entity_index, entity_move) in entities {
            self.entities[entity_index].pos = entity_move.new_pos;
        }
    }

    fn gravity(&self, entity_index: usize, _input: Input) -> Option<EntityMove> {
        if self.entity_magneted_angles(entity_index).next().is_some() {
            // No gravity when we have an active magnet
            return None;
        }
        if self
            .entity_active_effects(entity_index)
            .any(|(_, effect)| matches!(effect.deref(), Effect::DisableGravity))
        {
            // Or any DisableGravity effect is active
            return None;
        }
        let mut new_pos = self.entities[entity_index].pos;
        new_pos.cell.y -= 1;
        if !self.is_blocked(new_pos.cell) {
            return Some(EntityMove {
                used_input: Input::Skip,
                new_pos,
                move_type: EntityMoveType::Unsorted,
            });
        }
        None
    }

    fn entity_active_effects(
        &self,
        entity_index: usize,
    ) -> impl Iterator<Item = (IntAngle, Cow<Effect>)> + '_ {
        let entity = &self.entities[entity_index];
        let mut result = vec![];
        for (side_index, side) in entity.sides.iter().enumerate() {
            let side_angle = entity.side_angle(side_index);
            let side_cell = entity.pos.cell + side_angle.to_vec();
            if self.is_trigger(side_cell, side_angle.opposite()) {
                if let Some(effect) = &side.effect {
                    if effect.activate_self() {
                        result.push((side_angle, Cow::Borrowed(effect)));
                    }
                }
            }
            if self.is_trigger(entity.pos.cell, side_angle) {
                for other_entity in &self.entities {
                    if other_entity.pos.cell == side_cell {
                        let other_side_index = other_entity.side_index(side_angle.opposite());
                        let other_side = &other_entity.sides[other_side_index];
                        if let Some(effect) = &other_side.effect {
                            if let Some(effect_on_self) = effect.activate_other() {
                                result.push((side_angle, Cow::Owned(effect_on_self)));
                            }
                        }
                    }
                }
            }
        }
        result.into_iter()
    }

    fn entity_magneted_angles(&self, entity_index: usize) -> impl Iterator<Item = IntAngle> + '_ {
        self.entity_active_effects(entity_index)
            .flat_map(|(side, effect)| {
                if let Effect::Magnet = effect.deref() {
                    Some(side)
                } else {
                    None
                }
            })
    }

    fn just_move(&self, entity_index: usize, input: Input) -> Option<EntityMove> {
        if input == Input::Skip {
            return None;
        }
        let entity = &self.entities[entity_index];

        let magneted_angles: HashSet<IntAngle> = self
            .entity_magneted_angles(entity_index)
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
        if !self.is_blocked(entity.pos.cell + vec2(0, -1)) {
            left.move_dir = vec2::ZERO;
            right.move_dir = vec2::ZERO;
        }

        let find_magnet_direction = |f: &dyn Fn(IntAngle) -> IntAngle| {
            let mut possible = magneted_angles
                .iter()
                .map(|&angle| (angle, f(angle).normalize()))
                .filter(|(_, dir)| {
                    !magneted_angles.contains(dir)
                        && !self.is_blocked(entity.pos.cell + dir.to_vec())
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
        if !self.is_blocked(next_cell) {
            new_pos.cell = next_cell;
        }
        new_pos.angle = new_pos.angle.with_input(input);
        Some(EntityMove {
            used_input: input,
            new_pos,
            move_type: if let Some(magnet_angle) = direction.magnet_angle {
                EntityMoveType::Magnet {
                    magnet_angle,
                    move_dir: direction.move_dir,
                }
            } else {
                EntityMoveType::Unsorted
            },
        })
    }

    fn slide(&self, entity_index: usize, input: Input, side: IntAngle) -> Option<EntityMove> {
        if !side.is_down() {
            return None;
        }
        log::debug!("Sliding on {side:?}");

        let entity = &self.entities[entity_index];
        let input = entity.maybe_override_input(input);

        let new_pos = Position {
            cell: entity.pos.cell + vec2(input.delta(), 0),
            angle: entity.pos.angle,
        };
        if self.is_blocked(new_pos.cell) {
            return None;
        }
        Some(EntityMove {
            used_input: input,
            new_pos,
            move_type: EntityMoveType::Unsorted,
        })
    }

    fn jump_from(
        &self,
        entity_index: usize,
        input: Input,
        jump_from: IntAngle,
    ) -> Option<EntityMove> {
        log::debug!("Jumping from {jump_from:?}");

        let entity = &self.entities[entity_index];
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
            Some(EntityMove {
                used_input: input,
                new_pos,
                move_type: EntityMoveType::Unsorted,
            })
        } else {
            None
        }
    }

    fn side_effects(&self, entity_index: usize, input: Input) -> Option<EntityMove> {
        for (side, effect) in self.entity_active_effects(entity_index) {
            if let Some(pos) = effect.apply(self, entity_index, input, side) {
                return Some(pos);
            }
        }
        None
    }

    fn continue_magnet_move(&self, index: usize, input: Input) -> Option<EntityMove> {
        let entity = &self.entities[index];
        let Some(EntityMove {
            used_input: prev_input,
            move_type: EntityMoveType::Magnet {
                magnet_angle,
                move_dir,
            },
            ..
        }) = entity.prev_move else {
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
            cell: entity.pos.cell + magnet_angle.to_vec(),
            angle: entity.pos.angle.with_input(input),
        };
        if self.is_blocked(new_pos.cell) {
            return None;
        }
        Some(EntityMove {
            used_input: input,
            new_pos,
            move_type: EntityMoveType::Unsorted, // Can not continue magnet move more than 180
                                                 // degrees
        })
    }

    fn check_entity_move(&self, index: usize, input: Input) -> Option<EntityMove> {
        let systems: &[&dyn Fn(&Self, usize, Input) -> Option<EntityMove>] = &[
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
            entities: HashMap::new(),
        };
        for index in 0..self.entities.len() {
            if let Some(entity_move) = self.check_entity_move(
                index,
                if index == self.selected_player {
                    input
                } else {
                    Input::Skip
                },
            ) {
                result.entities.insert(index, entity_move);
            }
        }
        if result.entities.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    // TODO not return anything
    pub fn process_turn(&mut self, input: Input) -> Option<Moves> {
        self.process_powerups();
        let result = self.check_moves(input);
        for (entity_index, entity) in self.entities.iter_mut().enumerate() {
            entity.prev_pos = entity.pos;
            entity.prev_move = result
                .as_ref()
                .and_then(|moves| moves.entities.get(&entity_index))
                .cloned();
        }
        result
    }

    fn process_powerups(&mut self) {
        #[derive(Debug)]
        pub struct CollectedPowerup {
            pub entity: usize,
            pub entity_side: usize,
            pub powerup: usize,
        }

        let mut collected = Vec::new();
        for (entity_index, entity) in self.entities.iter().enumerate() {
            for (powerup_index, powerup) in self.powerups.iter().enumerate() {
                if entity.pos.cell != powerup.pos.cell {
                    continue;
                }
                let entity_side = entity.side_index(powerup.pos.angle);
                if entity.sides[entity_side].effect.is_none() {
                    collected.push(CollectedPowerup {
                        entity: entity_index,
                        entity_side: entity_side,
                        powerup: powerup_index,
                    })
                }
            }
        }
        for event in collected {
            let powerup = self.powerups.remove(event.powerup);
            let prev_effect = self.entities[event.entity].sides[event.entity_side]
                .effect
                .replace(powerup.effect);
            assert!(prev_effect.is_none());
        }
    }

    pub fn selected_entity(&self) -> &Entity {
        &self.entities[self.selected_player]
    }

    pub fn selected_entity_mut(&mut self) -> &mut Entity {
        &mut self.entities[self.selected_player]
    }

    pub fn finished(&self) -> bool {
        // TODO remove goal on entity touch, have goals.is_empty() here
        self.goals.iter().all(|goal| {
            self.entities
                .iter()
                .any(|entity| entity.pos.normalize() == goal.pos.normalize())
        })
    }
}
