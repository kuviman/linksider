use super::*;

#[derive(Debug)]
pub enum Event {
    CollectedPowerup {
        entity: Id,
        entity_side: usize,
        powerup: Id,
    },
    MoveStarted(EntityMove),
    MoveEnded(EntityMove),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityMoveType {
    Magnet {
        magnet_angle: IntAngle,
        move_dir: vec2<i32>,
    },
    EnterGoal {
        goal_id: Id,
    },
    Gravity,
    Move,
    Pushed,
    SlideStart,
    SlideContinue,
    Jump {
        from: IntAngle,
        blocked_angle: Option<IntAngle>,
        cells_traveled: usize,
        /// Number of cells that would be travelled if not blocked
        jump_force: usize,
    },
    MagnetContinue,
}

#[derive(Eq, PartialEq, HasId, Clone, Debug)]
pub struct EntityMove {
    #[has_id(id)]
    pub entity_id: Id,
    pub cells_reserved: HashSet<vec2<i32>>,
    pub start_time: Time,
    pub end_time: Time,
    pub used_input: Input,
    pub prev_pos: Position,
    pub new_pos: Position,
    pub move_type: EntityMoveType,
}

impl PartialOrd for EntityMove {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntityMove {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.end_time.cmp(&other.end_time) {
            core::cmp::Ordering::Equal => {}
            ord => return ord.reverse(),
        }
        self.entity_id.raw().cmp(&other.entity_id.raw())
    }
}
