use super::{level::Blocking, side::PickupSideEffects, *};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_player_input.in_set(OnUpdate(turns::State::WaitingForInput)));
        app.add_system(
            move_system
                .in_set(OnUpdate(turns::State::WaitingForInput))
                .after(update_player_input),
        );
        app.add_turn_system(falling_system, turns::TurnOrder::ApplySideEffects);

        app.register_ldtk_entity::<PlayerBundle>("Player");
        app.register_ldtk_entity::<BoxBundle>("Box");
    }
}

#[derive(Bundle, LdtkEntity)]
struct BoxBundle {
    blocking: level::Blocking,
    movable: Movable,
    trigger: side::Trigger,
    #[grid_coords]
    position: GridCoords,
    #[from_entity_instance]
    rotation: Rotation,
    #[from_entity_instance]
    entity_instance: EntityInstance,
    player_input: player::Input, // TODO remove
    pickup: PickupSideEffects,
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    #[with(entity_name)]
    name: Name,
}

#[derive(Bundle, LdtkEntity)]
struct PlayerBundle {
    player: Player,
    pickup: PickupSideEffects,
    movable: Movable,
    #[from_entity_instance]
    index: PlayerIndex,
    blocking: level::Blocking,
    trigger: side::Trigger,
    #[grid_coords]
    position: GridCoords,
    #[from_entity_instance]
    rotation: Rotation,
    #[from_entity_instance]
    entity_instance: EntityInstance,
    player_input: player::Input,
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    #[with(entity_name)]
    name: Name,
}

#[derive(Default, Component)]
pub struct Movable;

#[derive(Default, Component)]
pub struct Player;

#[derive(Component)]
pub struct SelectedPlayer;

#[derive(Component)]
pub struct SlideMove;

#[derive(Component)]
pub struct OverrideGravity(pub Vec<IVec2>);

// TODO: this is used by jump & slide but should not
#[derive(Default, Component)]
pub struct Input {
    pub direction: Direction,
}

fn update_player_input(
    keyboard_input: Res<bevy::input::Input<KeyCode>>,
    players: Query<(Entity, &PlayerIndex, &GridCoords, Option<&SelectedPlayer>), With<Player>>,
    mut inputs: Query<&mut Input, With<SelectedPlayer>>,
    mut commands: Commands,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    let mut dir = 0;
    if keyboard_input.any_pressed([KeyCode::A, KeyCode::Left]) {
        dir -= 1;
    }
    if keyboard_input.any_pressed([KeyCode::D, KeyCode::Right]) {
        dir += 1;
    }
    for mut input in inputs.iter_mut() {
        input.direction = match dir.cmp(&0) {
            std::cmp::Ordering::Less => Direction::Left,
            std::cmp::Ordering::Equal => Direction::None,
            std::cmp::Ordering::Greater => Direction::Right,
        };
    }

    if !players.is_empty() {
        let mut dir = 0;
        if keyboard_input.any_just_pressed([KeyCode::Tab, KeyCode::W, KeyCode::Up]) {
            dir = 1;
        }
        if keyboard_input.any_just_pressed([KeyCode::S, KeyCode::Down]) {
            dir = -1;
        }

        if dir != 0 || inputs.is_empty() {
            let mut players: Vec<(&PlayerIndex, (i32, i32), Entity, bool)> = players
                .iter()
                .map(|(entity, index, coords, selected)| {
                    (index, (coords.x, coords.y), entity, selected.is_some())
                })
                .collect();
            players.sort();
            let selected = players.iter().position(|&(.., selected)| selected);
            if let Some(selected) = selected {
                commands
                    .entity(players[selected].2)
                    .remove::<SelectedPlayer>();
            }
            let to_select = (selected.unwrap_or(0) as isize + players.len() as isize + dir)
                % players.len() as isize;
            let new_selected_player = players[to_select as usize].2;
            commands.entity(new_selected_player).insert(SelectedPlayer);
            audio.play_sfx(asset_server.load("sfx/selectPlayer.wav"));
            commands.spawn(VfxBundle::new(
                {
                    let (x, y) = players[to_select as usize].1;
                    GridCoords { x, y }
                },
                0,
                "animation/PLAYER_CHANGE.png",
                None,
                true,
                false,
            ));
        }
    }
}
#[allow(clippy::type_complexity)]
pub fn move_system(
    mut next_state: ResMut<NextState<turns::State>>,
    blocked: Query<BlockedQuery, With<Blocking>>,
    players: Query<
        (
            Entity,
            &Input,
            &GridCoords,
            &Rotation,
            Option<&OverrideGravity>,
            Option<&SlideMove>,
        ),
        With<SelectedPlayer>,
    >,
    mut events: EventWriter<turns::MoveEvent>,
) {
    for (player, input, coords, rot, override_gravity, slide_move) in players.iter() {
        let mut moved_to = *coords;
        let mut new_rotation = *rot;
        if input.direction == Direction::None {
            continue;
        }
        let mut ground_rot = 0;
        for &gravity_dir in
            override_gravity.map_or([IVec2::new(0, -1)].as_slice(), |g| g.0.as_slice())
        {
            let mut new_coords = IVec2::from(*coords);
            match input.direction {
                Direction::Left => new_coords += gravity_dir.rotate(IVec2::new(0, -1)),
                Direction::None => {
                    continue;
                }
                Direction::Right => new_coords += gravity_dir.rotate(IVec2::new(0, 1)),
            }
            let mut new_coords = new_coords.into();
            if is_blocked(new_coords, &blocked) {
                continue;
            }
            if override_gravity.is_some() {
                let turn_corner_coords = (IVec2::from(new_coords) + gravity_dir).into();
                if !is_blocked(turn_corner_coords, &blocked) {
                    new_coords = turn_corner_coords;
                    match input.direction {
                        Direction::Left => new_rotation.rotate_left(),
                        Direction::None => {
                            unreachable!()
                        }
                        Direction::Right => new_rotation.rotate_right(),
                    }
                }
            }
            moved_to = new_coords;
            ground_rot = vec_to_rot(gravity_dir);
            break;
        }
        if slide_move.is_none() || moved_to.x == coords.x {
            match input.direction {
                Direction::Left => new_rotation.rotate_left(),
                Direction::None => {
                    unreachable!()
                }
                Direction::Right => new_rotation.rotate_right(),
            }
            events.send(turns::MoveEvent {
                player,
                coords: moved_to,
                rotation: new_rotation,
                sfx: Some(if override_gravity.is_some() {
                    "sfx/magnet.wav"
                } else {
                    "sfx/move.wav"
                }),
                end_sfx: None,
                vfx: Some(VfxBundle::new(
                    *coords,
                    ground_rot,
                    "animation/walk.png",
                    None,
                    false,
                    input.direction == Direction::Left,
                )),
                end_vfx: None,
            });
            next_state.set(turns::State::Animation);
        } else {
            next_state.set(turns::State::Turn);
        }
    }
}

pub fn falling_system(
    blocked: Query<BlockedQuery, With<Blocking>>,
    players: Query<
        (
            Entity,
            &GridCoords,
            &Rotation,
            Option<&player::OverrideGravity>,
        ),
        With<Movable>,
    >,
    mut events: EventWriter<turns::MoveEvent>,
) {
    for (player, coords, rotation, override_gravity) in players.iter() {
        for &gravity in override_gravity.map_or([IVec2::new(0, -1)].as_slice(), |g| g.0.as_slice())
        {
            let new_coords = (IVec2::from(*coords) + gravity).into();
            if !is_blocked(new_coords, &blocked) {
                events.send(turns::MoveEvent {
                    player,
                    coords: new_coords,
                    rotation: *rotation,
                    sfx: None,
                    end_sfx: None,
                    vfx: None,
                    end_vfx: None,
                });
            }
        }
    }
}
