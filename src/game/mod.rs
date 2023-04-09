use std::f32::consts::PI;

use bevy::{
    ecs::query::{ReadOnlyWorldQuery, WorldQuery},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_ecs_ldtk::{prelude::*, utils::grid_coords_to_translation};

mod goal;
mod side;

pub struct Plugin;

#[derive(Default, Component)]
struct Player;

#[derive(Component)]
struct SelectedPlayer;

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    LoadingLevel,
    Turn,
    WaitingForInput,
    Animation,
}

// TODO: load from ldtk
// const BLOCK: i32 = 1;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(level_restart)
            .add_startup_system(music)
            .insert_resource(LevelSelection::Index(0))
            .insert_resource(LdtkSettings {
                set_clear_color: SetClearColor::FromLevelBackground,
                ..Default::default()
            })
            .register_ldtk_entity::<PlayerBundle>("Player")
            .add_state::<GameState>()
            .add_system(update_camera.after(update_transforms))
            .add_system(update_player_input.in_set(OnUpdate(GameState::WaitingForInput)))
            .add_system(
                player_move
                    .in_set(OnUpdate(GameState::WaitingForInput))
                    .after(update_player_input),
            )
            .add_system(start_animation.in_schedule(OnEnter(GameState::Animation)))
            .add_system(stop_animation.in_set(OnUpdate(GameState::Animation)))
            .add_system(process_animation.in_set(OnUpdate(GameState::Animation)));

        app.add_systems(
            (falling_system,)
                .in_set(OnUpdate(GameState::Turn))
                .before(end_turn),
        );
        app.add_system(end_turn.in_set(OnUpdate(GameState::Turn)));

        app.add_system(init_prev_coords.in_schedule(OnEnter(GameState::Turn)));
        app.add_system(update_transforms.in_set(OnUpdate(GameState::Animation)));
        // .register_ldtk_entity::<PowerupBundle<side::effects::slide::Effect>>("SlidePower");

        app.add_event::<MoveEvent>();

        side::init(app);
        goal::init(app);

        app.add_system(loading_level_finish);

        app.add_system(change_levels);

        app.register_ldtk_int_cell::<BlockBundle>(1);
        app.register_ldtk_int_cell::<WoodBundle>(6);
    }
}

#[derive(Default, Component)]
struct Blocking;

#[derive(Bundle, LdtkIntCell)]
struct BlockBundle {
    blocking: Blocking,
    trigger: side::Trigger,
}

#[derive(Bundle, LdtkIntCell)]
struct WoodBundle {
    blocking: Blocking,
}

fn loading_level_finish(
    mut next_state: ResMut<NextState<GameState>>,
    query: Query<(), Added<Handle<LdtkLevel>>>,
) {
    if !query.is_empty() {
        next_state.set(GameState::Turn);
    }
}

#[derive(Debug, Default, Component, Clone, Copy)]
pub struct Rotation(i32);

impl Rotation {
    pub fn to_radians(self) -> f32 {
        self.0 as f32 * PI / 2.0
    }
    pub fn rotate_right(&mut self) {
        self.0 += 3;
        self.0 %= 4;
    }
    pub fn rotate_left(&mut self) {
        self.0 += 1;
        self.0 %= 4;
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

#[derive(Component, Ord, PartialOrd, PartialEq, Eq)]
struct PlayerIndex(i32);

impl From<&EntityInstance> for PlayerIndex {
    fn from(entity: &EntityInstance) -> Self {
        match entity
            .field_instances
            .iter()
            .find(|field| field.identifier.to_lowercase() == "index")
            .expect("Set up player index daivy thx <3")
            .value
        {
            FieldValue::Int(Some(index)) => PlayerIndex(index),
            _ => panic!("Player index should be non null int"),
        }
    }
}

#[derive(Bundle, LdtkEntity)]
struct PlayerBundle {
    player: Player,
    #[from_entity_instance]
    index: PlayerIndex,
    blocking: Blocking,
    trigger: side::Trigger,
    #[grid_coords]
    position: GridCoords,
    rotation: Rotation,
    #[from_entity_instance]
    entity_instance: EntityInstance,
    player_input: PlayerInput,
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    #[with(entity_name)]
    name: Name,
}

fn entity_name(instance: &EntityInstance) -> Name {
    Name::new(instance.identifier.clone())
}

#[derive(Bundle, LdtkEntity)]
struct PowerupBundle<T: 'static + Send + Sync + Component + Default> {
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    #[grid_coords]
    position: GridCoords,
    effect: T,
    powerup: side::Powerup,
    #[with(entity_name)]
    name: Name,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load("world.ldtk"),
            // transform: Transform::from_scale(Vec3::splat(1.0 / 8.0)),
            ..default()
        },
        Name::new("World"),
    ));
    commands.spawn({
        let mut bundle = Camera2dBundle::default();
        bundle.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(200.0);
        bundle
    });
}

fn music(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio.play_with_settings(
        asset_server.load("game_music.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: 0.5,
            speed: 1.0,
        },
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    None,
    Right,
}

impl Direction {
    fn delta(&self) -> i32 {
        match self {
            Direction::Left => -1,
            Direction::None => 0,
            Direction::Right => 1,
        }
    }
}

impl Default for Direction {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default, Component)]
pub struct PlayerInput {
    pub direction: Direction,
}

fn update_player_input(
    keyboard_input: Res<Input<KeyCode>>,
    players: Query<(Entity, &PlayerIndex, &GridCoords, Option<&SelectedPlayer>), With<Player>>,
    mut inputs: Query<&mut PlayerInput, With<SelectedPlayer>>,
    mut commands: Commands,
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
        }
    }
}

fn update_camera(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    player: Query<&GlobalTransform, With<SelectedPlayer>>,
) {
    let mut camera = camera.single_mut();
    camera.translation = {
        let (sum, num) = player
            .iter()
            .fold((Vec2::ZERO, 0), |(sum, num), transform| {
                (sum + transform.translation().xy(), num + 1)
            });
        if num == 0 {
            warn!("No players??");
            return;
        }
        (sum / num as f32).extend(camera.translation.z)
    };
}

fn level_restart(
    ldtk_worlds: Query<Entity, With<Handle<LdtkAsset>>>,
    input: Res<Input<KeyCode>>,
    mut commands: Commands,
) {
    if input.just_released(KeyCode::R) {
        let ldtk_world = ldtk_worlds.single();
        commands.entity(ldtk_world).insert(Respawn);
    }
}

fn side_vec(player_rot: i32, side_rot: i32) -> IVec2 {
    match (player_rot + 4 - side_rot) % 4 {
        0 => IVec2::new(0, -1),
        1 => IVec2::new(1, 0),
        2 => IVec2::new(0, 1),
        3 => IVec2::new(-1, 0),
        _ => unreachable!(),
    }
}

#[derive(Component)]
struct SlideMove;

#[allow(clippy::type_complexity)]
fn player_move(
    mut next_state: ResMut<NextState<GameState>>,
    blocked: Query<BlockedQuery>,
    players: Query<
        (
            Entity,
            &PlayerInput,
            &GridCoords,
            &Rotation,
            Option<&OverrideGravity>,
            Option<&SlideMove>,
        ),
        With<SelectedPlayer>,
    >,
    mut events: EventWriter<MoveEvent>,
) {
    for (player, input, coords, rot, override_gravity, slide_move) in players.iter() {
        let mut moved_to = *coords;
        let mut new_rotation = *rot;
        if input.direction == Direction::None {
            continue;
        }
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
            events.send(MoveEvent(player, moved_to, new_rotation));
            next_state.set(GameState::Animation);
        } else {
            next_state.set(GameState::Turn);
        }
    }
}

#[derive(Component)]
struct PrevCoords(GridCoords);

#[derive(Component)]
struct PrevRotation(Rotation);

fn update_transforms(
    timer: Res<TurnAnimationTimer>,
    mut query: Query<(
        &PrevCoords,
        &GridCoords,
        &PrevRotation,
        &Rotation,
        &mut Transform,
    )>,
) {
    for (prev_coords, coords, prev_rot, rot, mut transform) in query.iter_mut() {
        let t = timer.0.elapsed_secs() / timer.0.duration().as_secs_f32();

        let prev_coords = &prev_coords.0;
        let tile_size = IVec2::new(16, 16); // TODO load from ldtk
        let prev_coords = grid_coords_to_translation(*prev_coords, tile_size);
        let coords = grid_coords_to_translation(*coords, tile_size);
        let interpolated_coords = prev_coords * (1.0 - t) + coords * t;
        transform.translation.x = interpolated_coords.x;
        transform.translation.y = interpolated_coords.y;

        let prev_rot = &prev_rot.0;
        let prev_rot = prev_rot.to_radians();
        let rot = rot.to_radians();
        let mut rot_diff = rot - prev_rot;
        while rot_diff > PI {
            rot_diff -= 2.0 * PI;
        }
        while rot_diff < -PI {
            rot_diff += 2.0 * PI;
        }
        let interpolated_rot = prev_rot + rot_diff * t;
        transform.rotation = Quat::from_rotation_z(interpolated_rot);
    }
}

fn init_prev_coords(
    query: Query<(Entity, &GridCoords, &Rotation), With<Player>>,
    mut commands: Commands,
) {
    for (entity, position, rot) in query.iter() {
        commands
            .entity(entity)
            .insert(PrevCoords(*position))
            .insert(PrevRotation(*rot));
    }
}

#[derive(Component)]
struct OverrideGravity(Vec<IVec2>);

fn falling_system(
    blocked: Query<BlockedQuery>,
    players: Query<(Entity, &GridCoords, &Rotation, Option<&OverrideGravity>), With<Player>>,
    mut events: EventWriter<MoveEvent>,
) {
    for (player, coords, rotation, override_gravity) in players.iter() {
        for &gravity in override_gravity.map_or([IVec2::new(0, -1)].as_slice(), |g| g.0.as_slice())
        {
            let new_coords = (IVec2::from(*coords) + gravity).into();
            if !is_blocked(new_coords, &blocked) {
                events.send(MoveEvent(player, new_coords, *rotation));
            }
        }
    }
}

fn end_turn(mut next_state: ResMut<NextState<GameState>>, events: EventReader<MoveEvent>) {
    if events.is_empty() {
        info!("Waiting for input now");
        next_state.set(GameState::WaitingForInput);
    } else {
        next_state.set(GameState::Animation);
    }
}

#[derive(Resource)]
struct TurnAnimationTimer(Timer);

#[derive(Debug)]
pub struct MoveEvent(pub Entity, pub GridCoords, pub Rotation);

fn start_animation(
    mut coords: Query<(&mut GridCoords, &mut Rotation)>,
    mut events: EventReader<MoveEvent>,
    mut commands: Commands,
) {
    info!("Animation started");
    for event in events.iter() {
        if let Ok((mut coords, mut rot)) = coords.get_mut(event.0) {
            *coords = event.1;
            *rot = event.2;
        }
    }
    commands.insert_resource(TurnAnimationTimer(Timer::from_seconds(
        0.2,
        TimerMode::Once,
    )));
}

fn stop_animation(
    mut next_state: ResMut<NextState<GameState>>,
    turn_timer: Res<TurnAnimationTimer>,
) {
    if turn_timer.0.finished() {
        info!("Animation finished");
        next_state.set(GameState::Turn);
    }
}

fn process_animation(mut turn_timer: ResMut<TurnAnimationTimer>, time: Res<Time>) {
    turn_timer.0.tick(time.delta()).elapsed_secs();
}

fn change_levels(input: Res<Input<KeyCode>>, mut level: ResMut<LevelSelection>) {
    let mut dir: isize = 0;
    if input.just_pressed(KeyCode::LBracket) {
        dir -= 1;
    }
    if input.just_pressed(KeyCode::RBracket) {
        dir += 1;
    }
    if dir != 0 {
        match *level {
            LevelSelection::Index(ref mut index) => {
                *index = (*index as isize + dir).max(0) as usize;
            }
            _ => unreachable!(),
        }
    }
}

#[derive(WorldQuery)]
struct BlockedQuery {
    coords: &'static GridCoords,
    filter: With<Blocking>,
}

fn is_blocked(coords: GridCoords, query: &Query<BlockedQuery, impl ReadOnlyWorldQuery>) -> bool {
    // TODO: bad performance
    query.iter().any(|item| item.coords == &coords)
}
