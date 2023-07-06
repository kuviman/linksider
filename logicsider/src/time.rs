use super::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Time {
    value: R32, // TODO maybe rationals?
}

impl Time {
    pub const ZERO: Self = Self { value: R32::ZERO };
    pub const ONE: Self = Self { value: R32::ONE };

    pub fn as_secs_f32(&self) -> f32 {
        self.value.as_f32()
    }

    pub fn from_secs_f32(secs: f32) -> Self {
        Self { value: r32(secs) }
    }
}

impl Add for Time {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
        }
    }
}
