use crate::game::level::Blocking;

use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.register_side_effect::<Jump>("JumpPower");
        // TODO: make this unsafe { }
        // EDIT: this is now unsafe POG
        #[allow(unused_unsafe)]
        unsafe {
            app.add_turn_system(do_jump.after(player::falling_system)); // After falling makes it have higher priority
        }
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
    players: Query<(&player::Input, &GridCoords, &Rotation)>,
    mut events: EventReader<SideEffectEvent<Jump>>,
    mut move_events: EventWriter<turns::MoveEvent>,
    blocked: Query<BlockedQuery, With<Blocking>>,
) {
    for event in events.iter() {
        if let Ok((player_input, player_coords, player_rotation)) = players.get(event.player) {
            let jump_dir = -side_vec(player_rotation.0, event.side);
            let mut path: Vec<IVec2> = vec![IVec2::new(1, 0)];
            if jump_dir == IVec2::new(0, 1) {
                path.push(IVec2::new(2, -player_input.direction.delta()));
                path.push(IVec2::new(2, -player_input.direction.delta() * 2));
            } else {
                path.push(IVec2::new(2, 0));
            }
            let path = path
                .into_iter()
                .map(|v| IVec2::from(*player_coords) + jump_dir.rotate(v));
            let mut path = Vec::from_iter(path);
            let mut hit_wall = false;
            if let Some(index) = 'find_block: {
                let mut prev_pos = IVec2::from(*player_coords);
                for index in 0..path.len() {
                    let delta = path[index] - prev_pos;
                    for dir in [(1, 1), (0, 1), (1, 0)] {
                        let delta = delta * IVec2::new(dir.0, dir.1);
                        if delta == IVec2::ZERO {
                            continue;
                        }
                        if is_blocked(GridCoords::from(prev_pos + delta), &blocked) {
                            break 'find_block Some(index);
                        }
                    }
                    prev_pos = path[index];
                }
                None
            } {
                path.truncate(index);
                if index < 2 {
                    // Not for the side
                    hit_wall = true;
                }
            }

            if let Some(last) = path.pop() {
                let last = last.into();
                move_events.send(turns::MoveEvent {
                    player: event.player,
                    coords: last,
                    rotation: if jump_dir == IVec2::new(0, 1) {
                        player_rotation
                            .rotated(player_input.direction)
                            .rotated(player_input.direction)
                    } else {
                        *player_rotation
                    },
                    sfx: Some("sfx/jump.wav"),
                    end_sfx: hit_wall.then_some("sfx/hitWall.wav"),
                    vfx: Some(VfxBundle::new(
                        *player_coords,
                        vec_to_rot(-jump_dir),
                        "animation/jump.png",
                        None,
                        true,
                        false,
                    )),
                    end_vfx: hit_wall.then_some(VfxBundle::new(
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
