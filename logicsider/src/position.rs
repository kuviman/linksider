use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub cell: vec2<i32>,
    pub angle: IntAngle,
}

impl Position {
    pub fn from_ldtk_entity(entity: &ldtk_json::EntityInstance, down_angle: IntAngle) -> Self {
        Self {
            cell: vec2(entity.grid[0], -entity.grid[1]),
            angle: entity
                .field_instances
                .iter()
                .find(|field| field.identifier == "Side")
                .map_or(IntAngle::DOWN, |field| {
                    match field.value.as_str().expect("Side value not a string WTF") {
                        "Down" => IntAngle::DOWN,
                        "Right" => IntAngle::RIGHT,
                        "Left" => IntAngle::LEFT,
                        "Up" => IntAngle::UP,
                        value => unreachable!("Unexpected side value {value:?}"),
                    }
                })
                - IntAngle::DOWN
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
