use super::*;

fn migrate_goal(Goal { id: _, pos }: Goal) -> v1::Goal {
    v1::Goal { pos }
}

fn migrate_powerup(Powerup { id: _, pos, effect }: Powerup) -> v1::Powerup {
    v1::Powerup { pos, effect }
}

fn migrate_entity(
    Entity {
        id: _,
        index,
        identifier,
        properties: _,
        pos,
        prev_pos: _,
        prev_move: _,
        sides,
    }: Entity,
) -> v1::Entity {
    v1::Entity {
        index,
        identifier,
        pos,
        sides,
    }
}

pub fn migrate(
    GameState {
        id_gen: _,
        tiles,
        entities,
        powerups,
        selected_player: _,
        goals,
        stable: _,
    }: GameState,
) -> v1::Level {
    let mut entities: Vec<v1::Entity> = entities.into_iter().map(migrate_entity).collect();
    for (cell, tile) in tiles {
        let identifier = match tile {
            Tile::Nothing => continue,
            Tile::Block => "block",
            Tile::Disable => "disable",
            Tile::Cloud => "cloud",
        };
        let pos = Position {
            cell,
            angle: IntAngle::RIGHT,
        };
        entities.push(v1::Entity {
            index: None,
            identifier: identifier.to_owned(),
            pos,
            sides: std::array::from_fn(|_| Side { effect: None }),
        });
    }
    v1::Level {
        entities,
        powerups: powerups.into_iter().map(migrate_powerup).collect(),
        goals: goals.into_iter().map(migrate_goal).collect(),
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tile {
    Nothing,
    Block,
    Disable,
    Cloud,
}

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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Properties {
    pub block: bool,
    pub trigger: bool,
    pub player: bool,
    pub pushable: bool,
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
