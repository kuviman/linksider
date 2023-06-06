use super::*;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameState {
    pub id_gen: id::Gen,
    pub tiles: HashMap<vec2<i32>, Tile>,
    pub entities: Collection<Entity>,
    pub powerups: Collection<Powerup>,
    pub selected_player: Option<Id>,
    pub goals: Collection<Goal>,
    pub stable: bool,
}

impl GameState {
    // TODO remove this, create separate level file format
    pub fn init_after_load(&mut self) {
        let first_player = self.player_ids().next();
        self.selected_player = first_player;
    }

    pub fn tile(&self, pos: vec2<i32>) -> Tile {
        self.tiles.get(&pos).copied().unwrap_or(Tile::Nothing)
    }

    pub fn center(&self) -> vec2<f32> {
        Aabb2::points_bounding_box(self.tiles.keys().copied())
            .extend_positive(vec2::splat(1))
            .map(|x| x as f32)
            .center()
    }

    pub fn add_entity(&mut self, identifier: &str, properties: &Properties, pos: Position) {
        self.entities.insert(Entity {
            id: self.id_gen.gen(),
            index: None,
            identifier: identifier.to_owned(),
            properties: properties.clone(),
            sides: std::array::from_fn(|_| Side { effect: None }),
            pos,
            prev_pos: pos,
            prev_move: None,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tile {
    Nothing, // TODO remove?
    Block,
    Disable,
    Cloud,
}

impl Tile {
    pub fn iter_variants() -> impl Iterator<Item = Self> {
        [Self::Block, Self::Disable, Self::Cloud].into_iter()
    }
}

/// Box entity
#[derive(Clone, PartialEq, Eq, HasId, Serialize, Deserialize)]
pub struct Entity {
    pub id: Id,
    pub index: Option<i32>, // for sorting
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Properties {
    pub block: bool,
    pub trigger: bool,
    pub player: bool,
    pub pushable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Side {
    pub effect: Option<Effect>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum Effect {
    Jump,
    Slide,
    Magnet,
    DisableGravity, // Fake effect (other side of magnet)
    DisableTrigger,
}

// TODO derive
impl Effect {
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
    pub fn iter_variants() -> impl Iterator<Item = Self> {
        [Self::Jump, Self::Magnet, Self::Slide, Self::DisableTrigger].into_iter()
    }
}

#[derive(Clone, PartialEq, Eq, HasId, Serialize, Deserialize)]
pub struct Goal {
    pub id: Id,
    pub pos: Position,
}

#[derive(Clone, PartialEq, Eq, HasId, Serialize, Deserialize)]
pub struct Powerup {
    pub id: Id,
    pub pos: Position,
    pub effect: Effect,
}
