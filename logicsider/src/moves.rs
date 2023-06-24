use super::*;

#[derive(Debug)]
pub struct CollectedPowerup {
    pub entity: Id,
    pub entity_side: usize,
    pub powerup: Id,
}

#[derive(Debug, Default)]
pub struct Moves {
    pub entity_moves: Collection<EntityMove>,
    pub collected_powerups: Vec<CollectedPowerup>,
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

#[derive(Debug, PartialEq, Eq, Clone, HasId, Serialize, Deserialize)]
pub struct EntityMove {
    #[has_id(id)]
    pub entity_id: Id,
    pub used_input: Input,
    pub prev_pos: Position,
    pub new_pos: Position,
    pub move_type: EntityMoveType,
}
