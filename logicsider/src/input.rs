use super::*;

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
