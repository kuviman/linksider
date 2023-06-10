use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
