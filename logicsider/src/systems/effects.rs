use super::*;

impl Effect {
    /// Whether having this effect enables or disables other effects on the touching side
    pub fn allow_trigger(&self) -> bool {
        match self {
            Self::DisableTrigger => false,
            _ => true,
        }
    }
    pub fn activate_self(&self) -> bool {
        true
    }
    pub fn activate_other(&self) -> Option<Self> {
        match self {
            Self::Jump | Self::Slide => Some(self.clone()),
            Self::Magnet => Some(Self::DisableGravity),
            Self::DisableGravity | Self::DisableTrigger => None,
        }
    }
    fn apply(
        &self,
        state: &GameState,
        entity_id: Id,
        input: Input,
        angle: IntAngle,
    ) -> Option<EntityMove> {
        match self {
            Self::Jump => jump::system(state, entity_id, input, angle),
            Self::Slide => slide::system(state, entity_id, input, angle),
            // Some effects are handled in other systems
            Self::Magnet | Self::DisableTrigger | Self::DisableGravity => None,
        }
    }
}

impl Tile {
    pub fn is_trigger(&self) -> bool {
        match self {
            Self::Nothing => false,
            Self::Block => true,
            Self::Disable => false,
            Self::Cloud => true,
        }
    }
}

pub fn is_trigger(state: &GameState, pos: vec2<i32>, angle: IntAngle) -> bool {
    state.tile(pos).is_trigger()
        || state.entities.iter().any(|entity| {
            entity.pos.cell == pos
                && entity.properties.trigger
                && entity
                    .side_at_angle(angle)
                    .effect
                    .as_ref()
                    .map_or(true, Effect::allow_trigger)
        })
}

pub fn side_effects(
    EntityMoveParams {
        state,
        entity_id,
        input,
        ..
    }: EntityMoveParams,
) -> Option<EntityMove> {
    for (side, effect) in entity_active_effects(state, entity_id) {
        if let Some(pos) = effect.apply(state, entity_id, input, side) {
            return Some(pos);
        }
    }
    None
}

pub fn entity_active_effects(
    state: &GameState,
    entity_id: Id,
) -> impl Iterator<Item = (IntAngle, Cow<Effect>)> + '_ {
    let entity = state.entities.get(&entity_id).unwrap();
    let mut result = vec![];
    for (side_index, side) in entity.sides.iter().enumerate() {
        let side_angle = entity.side_angle(side_index);
        let side_cell = entity.pos.cell + side_angle.to_vec();
        if is_trigger(state, side_cell, side_angle.opposite()) {
            if let Some(effect) = &side.effect {
                if effect.activate_self() {
                    result.push((side_angle, Cow::Borrowed(effect)));
                }
            }
        }
        if is_trigger(state, entity.pos.cell, side_angle) {
            for other_entity in &state.entities {
                if other_entity.pos.cell == side_cell {
                    let other_side_index = other_entity.side_index(side_angle.opposite());
                    let other_side = &other_entity.sides[other_side_index];
                    if let Some(effect) = &other_side.effect {
                        if let Some(effect_on_state) = effect.activate_other() {
                            result.push((side_angle, Cow::Owned(effect_on_state)));
                        }
                    }
                }
            }
        }
    }
    result.into_iter()
}
