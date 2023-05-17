use super::*;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    #[default]
    None,
    Right,
}

impl Direction {
    pub fn delta(&self) -> i32 {
        match self {
            Direction::Left => -1,
            Direction::None => 0,
            Direction::Right => 1,
        }
    }
}

impl std::ops::Neg for Direction {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::None => Self::None,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Debug, Default, Component, Clone, Copy)]
pub struct Rotation(pub i32);

impl From<&ldtk::EntityInstance> for Rotation {
    fn from(entity: &ldtk::EntityInstance) -> Self {
        let value = entity
            .field_instances
            .iter()
            .find(|field| field.identifier == "Side")
            .map(|field| match &field.value {
                FieldValue::Enum(value) => {
                    match value.as_ref().expect("Side should be non null").as_str() {
                        "Down" => 0,
                        "Right" => 1,
                        "Up" => 2,
                        "Left" => 3,
                        other => panic!("Unexpected value for side: {other:?}"),
                    }
                }
                _ => panic!("Side should be enum"),
            })
            .unwrap_or(0);
        Self(value)
    }
}

impl Rotation {
    pub fn to_radians(self) -> f32 {
        self.0 as f32 * PI / 2.0
    }
    pub fn rotate_right(&mut self) {
        self.0 -= 1;
    }
    pub fn rotate_left(&mut self) {
        self.0 += 1;
    }

    pub fn rotated(&self, direction: Direction) -> Self {
        let mut res = *self;
        match direction {
            Direction::Left => res.rotate_left(),
            Direction::None => {}
            Direction::Right => res.rotate_right(),
        }
        res
    }
}

pub fn vec_to_rot(v: IVec2) -> i32 {
    if v.y < 0 {
        return 0;
    }
    if v.y > 0 {
        return 2;
    }
    if v.x > 0 {
        return 1;
    }
    if v.x < 0 {
        return 3;
    }
    unreachable!()
}

pub fn side_vec(player_rot: i32, side_rot: i32) -> IVec2 {
    // TODO why we have - not a + ???
    match (player_rot - side_rot).rem_euclid(4) {
        0 => IVec2::new(0, -1),
        1 => IVec2::new(1, 0),
        2 => IVec2::new(0, 1),
        3 => IVec2::new(-1, 0),
        _ => unreachable!(),
    }
}

pub fn player_side(player_rot: &Rotation, dir: IVec2) -> i32 {
    let player_rot = player_rot.0;
    let mut dir = -vec_to_rot(dir);
    if dir % 2 == 0 {
        dir += 2; // DIRTY HACKS
    }
    // player_rot - side_rot = dir
    player_rot - dir
}

/// This is a helper to see if a cell is blocked or not
/// add blocked: Query<BlockedQuery, With<Blocking>> to system, then us is_blocked(coords, &blocked)
///
/// This should probably be done instead by introducing a resource that maintains a grid
/// For faster access, and having systems in place to synchronize it
#[derive(WorldQuery)]
pub struct BlockedQuery {
    coords: &'static GridCoords,
}

pub fn is_blocked(
    coords: GridCoords,
    query: &Query<BlockedQuery, impl ReadOnlyWorldQuery>,
) -> bool {
    // TODO: bad performance
    query.iter().any(|item| item.coords == &coords)
}
