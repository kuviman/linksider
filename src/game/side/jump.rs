use super::*;

pub fn init(app: &mut App) {
    app.register_side_effect::<Jump>("JumpPower");
    // TODO: make this unsafe { }
    // EDIT: this is now unsafe POG
    #[allow(unused_unsafe)]
    unsafe {
        app.add_system(do_jump.before(end_turn));
    }
}

#[derive(Default, Component)]
pub struct Jump;

impl SideEffect for Jump {
    fn texture() -> &'static str {
        "side_effects/jump.png"
    }
}

fn do_jump(
    players: Query<(&PlayerInput, &GridCoords, &Rotation)>,
    mut events: EventReader<SideEffectEvent<Jump>>,
    mut move_events: EventWriter<MoveEvent>,
    cells: Query<(&GridCoords, &IntGridCell)>,
) {
    for event in events.iter() {
        if let Ok((player_input, player_coords, player_rotation)) = players.get(event.player) {
            let path = [(0, 1), (0, 2), (player_input.direction.delta(), 2)];
            let path = path
                .map(|(dx, dy)| IVec2::from(*player_coords) + IVec2::new(dx, dy))
                .map(GridCoords::from);
            let mut path = Vec::from_iter(path);
            if let Some(index) = path.iter().position(|coords| {
                let cell = cells.iter().find_map(|(cell_coords, cell)| {
                    if cell_coords == coords {
                        Some(cell.value)
                    } else {
                        None
                    }
                });
                cell == Some(BLOCK)
            }) {
                path.truncate(index);
            }

            if let Some(last) = path.pop() {
                move_events.send(MoveEvent(
                    event.player,
                    last,
                    player_rotation.rotated(player_input.direction),
                ));
            }
        }
    }
}
