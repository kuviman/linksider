use super::*;

pub fn init(app: &mut App) {
    app.register_side_effect::<Jump>("JumpPower");
    // TODO: make this unsafe { }
    // EDIT: this is now unsafe POG
    #[allow(unused_unsafe)]
    unsafe {
        app.add_system(do_jump.before(end_turn).after(falling_system));
    }
}

#[derive(Debug, Default, Component)]
pub struct Jump;

impl SideEffect for Jump {
    fn active_side() -> bool {
        // bevy pog
        true
    }
    fn active_above() -> bool {
        true
    }
}

fn do_jump(
    players: Query<(&PlayerInput, &GridCoords, &Rotation)>,
    mut events: EventReader<SideEffectEvent<Jump>>,
    mut move_events: EventWriter<MoveEvent>,
    blocked: Query<BlockedQuery>,
) {
    for event in events.iter() {
        if let Ok((player_input, player_coords, player_rotation)) = players.get(event.player) {
            let jump_dir = -side_vec(player_rotation.0, event.side);
            let mut path: Vec<IVec2> = vec![IVec2::new(1, 0), IVec2::new(2, 0)];
            if jump_dir == IVec2::new(0, 1) {
                path.push(IVec2::new(2, -player_input.direction.delta()));
            } else if is_blocked(
                (IVec2::from(*player_coords) + IVec2::new(0, -1)).into(),
                &blocked,
            ) {
                continue;
            }
            let path = path
                .into_iter()
                .map(|v| IVec2::from(*player_coords) + jump_dir.rotate(v))
                .map(GridCoords::from);
            let mut path = Vec::from_iter(path);
            let mut hit_wall = false;
            if let Some(index) = path.iter().position(|coords| is_blocked(*coords, &blocked)) {
                path.truncate(index);
                if index < 2 {
                    // Not for the side
                    hit_wall = true;
                }
            }

            if let Some(last) = path.pop() {
                move_events.send(MoveEvent {
                    player: event.player,
                    coords: last,
                    rotation: if jump_dir == IVec2::new(0, 1) {
                        player_rotation.rotated(player_input.direction)
                    } else {
                        *player_rotation
                    },
                    sfx: Some("sfx/jump.wav"),
                    end_sfx: hit_wall.then_some("sfx/hitWall.wav"),
                    vfx: Some(AnimationBundle::new(
                        *player_coords,
                        vec_to_rot(-jump_dir),
                        "animation/jump.png",
                        None,
                        true,
                        false,
                    )),
                    end_vfx: hit_wall.then_some(AnimationBundle::new(
                        last,
                        vec_to_rot(-jump_dir), // TODO
                        "animation/hit_wall.png",
                        None,
                        false,
                        false,
                    )),
                });
            }
        }
    }
}
