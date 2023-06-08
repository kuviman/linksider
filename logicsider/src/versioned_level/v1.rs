use super::*;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Level {
    pub entities: Vec<Entity>,
    pub powerups: Vec<Powerup>,
    pub goals: Vec<Goal>,
}

/// Box entity
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub index: Option<i32>, // for sorting
    pub identifier: String, // TODO remove
    pub pos: Position,
    pub sides: [Side; 4],
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Powerup {
    pub pos: Position,
    pub effect: Effect,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub pos: Position,
}
