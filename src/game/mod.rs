use std::f32::consts::PI;

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_ecs_ldtk::{prelude::*, utils::grid_coords_to_translation};

use self::config::Config;

pub mod config;
mod goal;
mod side;

pub struct Plugin;

impl Default for Config {
    fn default() -> Self {
        serde_json::from_str(include_str!("config.json")).unwrap()
    }
}

#[derive(Default, Component)]
struct Player;

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Turn,
    WaitingForInput,
    Animation,
}

// TODO: load from ldtk
const BLOCK: i32 = 1;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Config>()
            .register_type::<Config>()
            .add_plugin(bevy_inspector_egui::quick::ResourceInspectorPlugin::<Config>::default())
            .add_startup_system(setup)
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

#[derive(Bundle, LdtkEntity)]
struct PlayerBundle {
    player: Player,
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

#[derive(Clone, Copy)]
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
    mut inputs: Query<&mut PlayerInput, With<Player>>,
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
}

fn update_camera(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    player: Query<&GlobalTransform, With<Player>>,
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

fn player_move(
    mut next_state: ResMut<NextState<GameState>>,
    cells: Query<(&GridCoords, &IntGridCell)>,
    mut players: Query<(&PlayerInput, &mut GridCoords, &mut Rotation), Without<IntGridCell>>,
) {
    for (input, mut coords, mut rot) in players.iter_mut() {
        let mut new_coords = *coords;
        match input.direction {
            Direction::Left => new_coords.x -= 1,
            Direction::None => {
                continue;
            }
            Direction::Right => new_coords.x += 1,
        }
        // TODO: bad performance
        let cell = cells.iter().find_map(|(coords, cell)| {
            if coords == &new_coords {
                Some(cell)
            } else {
                None
            }
        });
        if cell.map_or(true, |cell| cell.value != BLOCK) {
            *coords = new_coords;
        }
        match input.direction {
            Direction::Left => rot.rotate_left(),
            Direction::None => {}
            Direction::Right => rot.rotate_right(),
        }
        next_state.set(GameState::Animation);
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

fn falling_system(
    cells: Query<(&GridCoords, &IntGridCell)>,
    players: Query<(Entity, &GridCoords, &Rotation), With<Player>>,
    mut events: EventWriter<MoveEvent>,
) {
    for (player, coords, rotation) in players.iter() {
        let mut new_coords = *coords;
        new_coords.y -= 1;
        // TODO: bad performance
        let cell = cells.iter().find_map(|(coords, cell)| {
            if coords == &new_coords {
                Some(cell)
            } else {
                None
            }
        });
        if cell.map_or(true, |cell| cell.value != BLOCK) {
            events.send(MoveEvent(player, new_coords, *rotation));
        }
    }
}

fn end_turn(
    mut next_state: ResMut<NextState<GameState>>,
    mut coords: Query<(&mut GridCoords, &mut Rotation)>,
    mut events: EventReader<MoveEvent>,
) {
    if events.is_empty() {
        info!("Waiting for input now");
        next_state.set(GameState::WaitingForInput);
    } else {
        info!("Animation started");
        for event in events.iter() {
            if let Ok((mut coords, mut rot)) = coords.get_mut(event.0) {
                *coords = event.1;
                *rot = event.2;
            }
        }
        next_state.set(GameState::Animation);
    }
}

#[derive(Resource)]
struct TurnAnimationTimer(Timer);

pub struct MoveEvent(pub Entity, pub GridCoords, pub Rotation);

fn start_animation(mut commands: Commands) {
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
