use super::*;

#[derive(Clone)]
pub struct GameState {
    pub id_gen: id::Gen,
    pub tiles: HashMap<vec2<i32>, Tile>,
    pub entities: Collection<Entity>,
    pub powerups: Collection<Powerup>,
    pub selected_player: Option<Id>,
    pub goals: Collection<Goal>,
    pub config: Config,
    pub stable: bool,
}

impl GameState {
    pub fn tile(&self, pos: vec2<i32>) -> Tile {
        self.tiles.get(&pos).copied().unwrap_or(Tile::Nothing)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Nothing,
    Block,
    Disable,
    Cloud,
}

/// Box entity
#[derive(Clone, HasId)]
pub struct Entity {
    pub id: Id,
    pub identifier: String, // TODO remove
    pub properties: Properties,
    pub pos: Position,
    pub prev_pos: Position,
    pub prev_move: Option<EntityMove>,
    pub sides: [Side; 4],
}

impl Entity {
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

#[derive(Debug, Clone)]
pub struct Properties {
    pub block: bool,
    pub trigger: bool,
    pub player: bool,
    pub pushable: bool,
}

#[derive(Clone, Debug)]
pub struct Side {
    pub effect: Option<Effect>,
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
}

#[derive(Clone, HasId)]
pub struct Goal {
    pub id: Id,
    pub pos: Position,
}

#[derive(Clone, HasId)]
pub struct Powerup {
    pub id: Id,
    pub pos: Position,
    pub effect: Effect,
}
