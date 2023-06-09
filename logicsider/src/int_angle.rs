use super::*;

// TODO move to batbox
#[derive(PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct IntAngle(i32);

impl Add for IntAngle {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Add<Input> for IntAngle {
    type Output = Self;
    fn add(self, rhs: Input) -> Self {
        Self(self.0 - rhs.delta())
    }
}

impl Sub for IntAngle {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for IntAngle {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl IntAngle {
    pub const RIGHT: Self = Self(0);
    pub const UP: Self = Self(1);
    pub const LEFT: Self = Self(2);
    pub const DOWN: Self = Self(3);

    pub fn normalize(&self) -> Self {
        Self(self.0.rem_euclid(4))
    }

    pub fn to_i32(&self) -> i32 {
        self.0
    }

    pub fn from_i32(value: i32) -> Self {
        Self(value)
    }

    pub fn opposite(&self) -> Self {
        Self(self.0 + 2)
    }

    pub fn is_right(&self) -> bool {
        self.normalize().0 == 0
    }

    pub fn is_up(&self) -> bool {
        self.normalize().0 == 1
    }

    pub fn is_left(&self) -> bool {
        self.normalize().0 == 2
    }

    pub fn is_down(&self) -> bool {
        self.normalize().0 == 3
    }

    pub fn to_angle(&self) -> Angle<f32> {
        Angle::from_degrees(self.0 as f32 * 90.0)
    }

    pub fn to_matrix(&self) -> mat3<f32> {
        mat3::rotate(self.to_angle())
    }

    pub fn with_input(self, input: Input) -> Self {
        match input {
            Input::Left => self.rotate_counter_clockwise(),
            Input::Skip => self,
            Input::Right => self.rotate_clockwise(),
        }
    }

    pub fn rotate_counter_clockwise(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn rotate_clockwise(self) -> Self {
        Self(self.0 - 1)
    }

    pub fn rotate_vec(&self, p: vec2<i32>) -> vec2<i32> {
        match self.normalize().0 {
            0 => p,
            1 => p.rotate_90(),
            2 => -p,
            3 => -p.rotate_90(),
            _ => unreachable!(),
        }
    }

    pub fn to_vec(&self) -> vec2<i32> {
        match self.normalize().0 {
            0 => vec2(1, 0),
            1 => vec2(0, 1),
            2 => vec2(-1, 0),
            3 => vec2(0, -1),
            _ => unreachable!(),
        }
    }

    pub fn try_from_vec(v: vec2<i32>) -> Option<Self> {
        match (v.x.signum(), v.y.signum()) {
            (-1, 0) => Some(Self::LEFT),
            (1, 0) => Some(Self::RIGHT),
            (0, 1) => Some(Self::UP),
            (0, -1) => Some(Self::DOWN),
            _ => None,
        }
    }

    pub fn from_vec(v: vec2<i32>) -> Self {
        Self::try_from_vec(v).unwrap()
    }
}

impl Debug for IntAngle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.normalize().0 {
            0 => write!(f, "Right"),
            1 => write!(f, "Up"),
            2 => write!(f, "Left"),
            3 => write!(f, "Down"),
            _ => unreachable!(),
        }
    }
}
