use super::*;

pub use versioned_level::current::{Entity, Goal, Level, Powerup};

impl Level {
    pub fn empty() -> Self {
        Self {
            entities: default(),
            powerups: default(),
            goals: default(),
        }
    }
    pub fn bounding_box(&self) -> Aabb2<i32> {
        Aabb2::points_bounding_box(self.entities.iter().map(|entity| entity.pos.cell))
            .unwrap_or(Aabb2::ZERO)
            .extend_positive(vec2::splat(1))
    }
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
