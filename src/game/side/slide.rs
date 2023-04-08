use super::*;

pub fn init(app: &mut App) {
    app.register_side_effect::<Slide>("SlidePower");
    app.add_system(do_slide.before(end_turn));
}

#[derive(Default, Component)]
pub struct Slide;

impl SideEffect for Slide {
    fn texture() -> &'static str {
        "side_effects/slide.png"
    }
}

fn do_slide(
    players: Query<(&PlayerInput, &GridCoords, &Rotation)>,
    mut events: EventReader<SideEffectEvent<Slide>>,
    mut move_events: EventWriter<MoveEvent>,
    cells: Query<(&GridCoords, &IntGridCell)>,
) {
    for event in events.iter() {
        let Ok((player_input, player_coords, player_rotation)) = players.get(event.player) else { continue };

        let next_pos = GridCoords {
            x: player_coords.x + player_input.direction.delta(),
            y: player_coords.y,
        };

        let cell = cells.iter().find_map(|(cell_coords, cell)| {
            if cell_coords == &next_pos {
                Some(cell.value)
            } else {
                None
            }
        });
        if cell == Some(BLOCK) {
            continue;
        }

        let below = GridCoords {
            x: next_pos.x,
            y: next_pos.y - 1,
        };
        let cell_below = cells.iter().find_map(|(cell_coords, cell)| {
            if cell_coords == &below {
                Some(cell.value)
            } else {
                None
            }
        });
        let mut next_rotation = *player_rotation;
        if cell_below != Some(BLOCK) {
            next_rotation = next_rotation.rotated(player_input.direction);
        }

        move_events.send(MoveEvent(event.player, next_pos, next_rotation));
    }
}
