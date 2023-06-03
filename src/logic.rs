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
pub struct CollectedPowerup {
    pub entity: Id,
    pub entity_side: usize,
    pub powerup: Id,
}

#[derive(Debug, Default)]
pub struct Moves {
    pub collected_powerups: Vec<CollectedPowerup>,
    pub entity_moves: Collection<EntityMove>,
}

#[derive(Debug, Clone)]
pub enum EntityMoveType {
    Magnet {
        magnet_angle: IntAngle,
        move_dir: vec2<i32>,
    },
    Unsorted, // TODO remove
    EnterGoal {
        goal_id: Id,
    },
}

#[derive(Debug, Clone, HasId)]
pub struct EntityMove {
    #[has_id(id)]
    pub entity_id: Id,
    pub used_input: Input,
    pub prev_pos: Position,
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
    pub fn allow_trigger(&self) -> bool {
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
        entity_id: Id,
        input: Input,
        angle: IntAngle,
    ) -> Option<EntityMove> {
        match self {
            Self::Jump => state.jump_from(entity_id, input, angle),
            Self::Slide => state.slide(entity_id, input, angle),
            // Some effects are handled in other systems
            Self::Magnet | Self::DisableTrigger | Self::DisableGravity => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Side {
    pub effect: Option<Effect>,
}

#[derive(Debug, Clone)]
pub struct Properties {
    pub block: bool,
    pub trigger: bool,
    pub player: bool,
    pub pushable: bool,
}

/// Box entity
#[derive(Clone, HasId)]
pub struct Entity {
    pub id: Id,
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

#[derive(Clone, HasId)]
pub struct Goal {
    pub id: Id,
    pub pos: Position,
    pub mesh: Rc<ldtk::Mesh>, // TODO should not be here
}

#[derive(Clone, HasId)]
pub struct Powerup {
    pub id: Id,
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

#[derive(Clone)]
pub struct GameState {
    id_gen: id::Gen,
    pub level: Rc<ldtk::Level>, // TODO remove, this is not state
    pub tiles: HashMap<vec2<i32>, Tile>,
    pub entities: Collection<Entity>,
    pub powerups: Collection<Powerup>,
    pub selected_player: Option<Id>,
    pub goals: Collection<Goal>,
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
            id_gen: id::Gen::new(),
            tiles,
            level: level.clone(),
            entities: default(),
            powerups: default(),
            goals: default(),
            selected_player: None,
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
                result.powerups.insert(Powerup {
                    id: result.id_gen.gen(),
                    effect: Effect::from_str(effect),
                    pos: Position::from_ldtk_entity(entity, IntAngle::DOWN),
                    mesh: entity.mesh.clone(),
                });
            } else {
                match entity.identifier.as_str() {
                    "Goal" => {
                        result.goals.insert(Goal {
                            id: result.id_gen.gen(),
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
                                pushable: false,
                            },
                            "Crate" => Properties {
                                block: true,
                                trigger: true,
                                player: false,
                                pushable: false,
                            },
                            "Box" => Properties {
                                block: true,
                                trigger: true,
                                player: false,
                                pushable: true,
                            },
                            "DisableBox" => Properties {
                                block: true,
                                trigger: false,
                                player: false,
                                pushable: true,
                            },
                            _ => unimplemented!("Entity {entity_name:?} unimplemented"),
                        };
                        result.entities.insert(Entity {
                            id: result.id_gen.gen(),
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
        result.selected_player = result
            .entities
            .iter()
            .filter(|entity| entity.properties.player)
            .min_by_key(|entity| entity.id.raw())
            .map(|entity| entity.id);
        result
    }

    pub fn change_player_selection(&mut self, delta: isize) {
        let mut player_ids: Vec<Id> = self
            .entities
            .iter()
            .filter_map(|entity| {
                if entity.properties.player {
                    Some(entity.id)
                } else {
                    None
                }
            })
            .collect();
        player_ids.sort_by_key(|id| id.raw());
        let index = player_ids
            .iter()
            .position(|&id| Some(id) == self.selected_player);
        let Some(index) = index else {
            self.selected_player = player_ids.first().copied();
            return;
        };
        let mut new = index as isize + delta;
        if new < 0 {
            new = player_ids.len() as isize - 1;
        } else if new >= player_ids.len() as isize {
            new = 0;
        }
        self.selected_player = Some(player_ids[usize::try_from(new).unwrap()]);
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
                    && entity.properties.trigger
                    && entity
                        .side_at_angle(angle)
                        .effect
                        .as_ref()
                        .map_or(true, Effect::allow_trigger)
            })
    }

    fn enter_goal(&self, entity_id: Id, _input: Input) -> Option<EntityMove> {
        let entity = self.entities.get(&entity_id).unwrap();
        // TODO not only players?
        if !entity.properties.player {
            return None;
        }
        if let Some(goal) = self
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

    fn gravity(&self, entity_id: Id, _input: Input) -> Option<EntityMove> {
        if self.entity_magneted_angles(entity_id).next().is_some() {
            // No gravity when we have an active magnet
            return None;
        }
        if self
            .entity_active_effects(entity_id)
            .any(|(_, effect)| matches!(effect.deref(), Effect::DisableGravity))
        {
            // Or any DisableGravity effect is active
            return None;
        }
        let entity = self.entities.get(&entity_id).unwrap();
        let mut new_pos = entity.pos;
        new_pos.cell.y -= 1;
        if !self.is_blocked(new_pos.cell) {
            return Some(EntityMove {
                entity_id: entity.id,
                used_input: Input::Skip,
                prev_pos: entity.pos,
                new_pos,
                move_type: EntityMoveType::Unsorted,
            });
        }
        None
    }

    fn entity_active_effects(
        &self,
        entity_id: Id,
    ) -> impl Iterator<Item = (IntAngle, Cow<Effect>)> + '_ {
        let entity = self.entities.get(&entity_id).unwrap();
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

    fn entity_magneted_angles(&self, entity_id: Id) -> impl Iterator<Item = IntAngle> + '_ {
        self.entity_active_effects(entity_id)
            .flat_map(|(side, effect)| {
                if let Effect::Magnet = effect.deref() {
                    Some(side)
                } else {
                    None
                }
            })
    }

    fn just_move(&self, entity_id: Id, input: Input) -> Option<Collection<EntityMove>> {
        if input == Input::Skip {
            return None;
        }
        let entity = self.entities.get(&entity_id).unwrap();

        let magneted_angles: HashSet<IntAngle> = self
            .entity_magneted_angles(entity_id)
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
        let mut result = Collection::new();
        if !self.is_blocked(next_cell) {
            new_pos.cell = next_cell;
        }
        if let Some(next_entity) = self
            .entities
            .iter()
            .find(|entity| entity.pos.cell == next_cell)
        {
            if next_entity.properties.pushable {
                let next_next_cell = next_cell + direction.move_dir;
                if !self.is_blocked(next_next_cell) {
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
                        move_type: EntityMoveType::Unsorted,
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
                EntityMoveType::Unsorted
            },
        });
        Some(result)
    }

    fn slide(&self, entity_id: Id, input: Input, side: IntAngle) -> Option<EntityMove> {
        if !side.is_down() {
            return None;
        }
        log::debug!("Sliding on {side:?}");

        let entity = self.entities.get(&entity_id).unwrap();

        let slide_with_input = |input: Input| -> Option<EntityMove> {
            let new_pos = Position {
                cell: entity.pos.cell + vec2(input.delta(), 0),
                angle: entity.pos.angle,
            };
            if self.is_blocked(new_pos.cell) {
                return None;
            }
            Some(EntityMove {
                entity_id: entity.id,
                used_input: input,
                prev_pos: entity.pos,
                new_pos,
                move_type: EntityMoveType::Unsorted,
            })
        };
        slide_with_input(entity.maybe_override_input(input)).or(slide_with_input(input))
    }

    fn jump_from(&self, entity_id: Id, input: Input, jump_from: IntAngle) -> Option<EntityMove> {
        log::debug!("Jumping from {jump_from:?}");

        let entity = self.entities.get(&entity_id).unwrap();
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
                entity_id: entity.id,
                used_input: input,
                prev_pos: entity.pos,
                new_pos,
                move_type: EntityMoveType::Unsorted,
            })
        } else {
            None
        }
    }

    fn side_effects(&self, entity_id: Id, input: Input) -> Option<EntityMove> {
        for (side, effect) in self.entity_active_effects(entity_id) {
            if let Some(pos) = effect.apply(self, entity_id, input, side) {
                return Some(pos);
            }
        }
        None
    }

    fn continue_magnet_move(&self, entity_id: Id, input: Input) -> Option<EntityMove> {
        let entity = self.entities.get(&entity_id).unwrap();
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
            entity_id: entity.id,
            used_input: input,
            prev_pos: entity.pos,
            new_pos,
            move_type: EntityMoveType::Unsorted, // Can not continue magnet move more than 180
                                                 // degrees
        })
    }

    fn check_entity_move(&self, entity_id: Id, input: Input) -> Option<Collection<EntityMove>> {
        macro_rules! system {
            ($f:expr) => {
                if let Some(moves) = $f(self, entity_id, input) {
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

        system!(simple(Self::continue_magnet_move));
        system!(simple(Self::side_effects));
        system!(simple(Self::gravity));
        system!(simple(Self::enter_goal));
        system!(Self::just_move);

        None
    }

    fn check_moves(&self, input: Input) -> Collection<EntityMove> {
        let mut result = Collection::new();
        for &id in self.entities.ids() {
            if let Some(moves) = self.check_entity_move(
                id,
                if Some(id) == self.selected_player {
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

    fn perform_moves(&mut self, moves: &Collection<EntityMove>) {
        for entity_move in moves {
            let entity = self.entities.get_mut(&entity_move.entity_id).unwrap();
            assert_eq!(entity.pos, entity_move.prev_pos);
            entity.pos = entity_move.new_pos;
            if let EntityMoveType::EnterGoal { goal_id } = entity_move.move_type {
                self.goals.remove(&goal_id);
                self.entities.remove(&entity_move.entity_id);
            }
        }
    }

    pub fn process_turn(&mut self, input: Input) -> Option<Moves> {
        let result = Moves {
            collected_powerups: self.process_powerups(),
            entity_moves: {
                let moves = self.check_moves(input);
                // TODO check for conflicts
                for entity in self.entities.iter_mut() {
                    entity.prev_pos = entity.pos;
                    entity.prev_move = moves.get(&entity.id).cloned();
                }
                self.perform_moves(&moves);
                moves
            },
        };
        if result.collected_powerups.is_empty() && result.entity_moves.is_empty() {
            return None;
        }
        Some(result)
    }

    fn process_powerups(&mut self) -> Vec<CollectedPowerup> {
        let mut collected = Vec::new();
        for entity in &self.entities {
            for powerup in &self.powerups {
                if entity.pos.cell != powerup.pos.cell {
                    continue;
                }
                let entity_side = entity.side_index(powerup.pos.angle);
                if entity.sides[entity_side].effect.is_none() {
                    collected.push(CollectedPowerup {
                        entity: entity.id,
                        entity_side: entity_side,
                        powerup: powerup.id,
                    })
                }
            }
        }
        for event in &collected {
            let powerup = self.powerups.remove(&event.powerup).unwrap();
            let prev_effect = self.entities.get_mut(&event.entity).unwrap().sides
                [event.entity_side]
                .effect
                .replace(powerup.effect);
            assert!(prev_effect.is_none());
        }
        collected
    }

    pub fn selected_entity(&self) -> Option<&Entity> {
        self.selected_player.and_then(|id| self.entities.get(&id))
    }

    pub fn finished(&self) -> bool {
        self.goals.is_empty()
    }
}
