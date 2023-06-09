use super::*;

#[derive(Clone)]
pub struct GameState {
    pub id_gen: id::Gen,
    pub entities: Collection<Entity>,
    pub powerups: Collection<Powerup>,
    pub selected_player: Option<Id>,
    pub goals: Collection<Goal>,
    pub stable: bool,
}

impl GameState {
    pub fn empty() -> Self {
        Self {
            id_gen: id::Gen::new(),
            entities: default(),
            powerups: default(),
            selected_player: None,
            goals: default(),
            stable: false,
        }
    }

    pub fn init(config: &Config, level: &Level) -> Self {
        let mut id_gen = id::Gen::new();
        let entities = level
            .entities
            .iter()
            .map(
                |&level::Entity {
                     index,
                     ref identifier,
                     pos,
                     ref sides,
                 }| Entity {
                    id: id_gen.gen(),
                    index,
                    identifier: identifier.clone(),
                    properties: config.entities.get(identifier).unwrap().clone(),
                    pos,
                    prev_pos: pos,
                    prev_move: None,
                    sides: sides.clone(),
                },
            )
            .collect();
        let powerups = level
            .powerups
            .iter()
            .map(|&level::Powerup { pos, ref effect }| Powerup {
                id: id_gen.gen(),
                pos,
                effect: effect.clone(),
            })
            .collect();
        let goals = level
            .goals
            .iter()
            .map(|&level::Goal { pos }| Goal {
                id: id_gen.gen(),
                pos,
            })
            .collect();
        let mut result = Self {
            id_gen,
            entities,
            powerups,
            selected_player: None,
            goals,
            stable: false,
        };
        let first_player = result.player_ids().next();
        result.selected_player = first_player;
        result
    }

    pub fn bounding_box(&self) -> Aabb2<i32> {
        Aabb2::points_bounding_box(self.entities.iter().map(|entity| entity.pos.cell))
            .unwrap_or(Aabb2::ZERO)
            .extend_positive(vec2::splat(1))
    }

    pub fn center(&self) -> vec2<f32> {
        self.bounding_box().map(|x| x as f32).center()
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

    pub fn side_at_angle_mut(&mut self, angle: IntAngle) -> &mut Side {
        &mut self.sides[self.side_index(angle)]
    }

    pub fn relative_side_angle(side_index: usize) -> IntAngle {
        IntAngle::from_i32(side_index as i32)
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Side {
    pub effect: Option<Effect>,
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Properties {
    pub block: bool,
    pub trigger: bool,
    pub player: bool,
    pub pushable: bool,
    pub r#static: bool,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum Effect {
    Jump,
    Slide,
    Magnet,
    WeakMagnet,
    DisableTrigger,
}

// TODO derive
impl Effect {
    pub fn from_str(name: &str) -> Self {
        match name {
            "Jump" => Self::Jump,
            "Magnet" => Self::Magnet,
            "Slide" => Self::Slide,
            "WeakMagnet" => Self::WeakMagnet,
            "DisableTrigger" => Self::DisableTrigger,
            _ => unimplemented!("{name:?} effect is unimplemented"),
        }
    }
    pub fn iter_variants() -> impl Iterator<Item = Self> {
        [Self::Jump, Self::Magnet, Self::Slide, Self::DisableTrigger].into_iter()
    }
}

#[derive(Clone, PartialEq, Eq, HasId, Serialize, Deserialize)]
pub struct Powerup {
    pub id: Id,
    pub pos: Position,
    pub effect: Effect,
}

#[derive(Clone, PartialEq, Eq, HasId, Serialize, Deserialize)]
pub struct Goal {
    pub id: Id,
    pub pos: Position,
}
