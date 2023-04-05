use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use std::f32::consts::PI;

use self::side::HasSides;

mod side;

pub struct Plugin;

#[derive(Default, Component)]
struct Player;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(update_player_input)
            .add_system(player_rotation_control)
            .add_system(update_camera)
            .add_startup_system(music)
            .insert_resource(LevelSelection::Index(0))
            // Required to prevent race conditions between bevy_ecs_ldtk's and bevy_rapier's systems
            .configure_set(LdtkSystemSet::ProcessApi.before(PhysicsSet::SyncBackend))
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                    load_level_neighbors: true,
                },
                set_clear_color: SetClearColor::FromLevelBackground,
                ..Default::default()
            })
            .register_ldtk_int_cell::<BlockBundle>(1)
            .add_system(spawn_wall_collision)
            .register_ldtk_entity::<PlayerBundle>("Player")
            .register_ldtk_entity::<PowerupBundle<side::effects::jump::Effect>>("JumpPower")
            .register_ldtk_entity::<PowerupBundle<side::effects::slide::Effect>>("SlidePower");
        side::init(app);
    }
}

#[derive(Default, Component)]
struct Block;

#[derive(Bundle, LdtkIntCell)]
struct BlockBundle {
    block: Block,
    // side_effect_trigger: side::Trigger,
    // #[with(block_collider)]
    // collider: Collider,
}

fn block_collider(_: IntGridCell) -> Collider {
    Collider::cuboid(8.0, 8.0)
}

/// Copypasta from https://github.com/Trouv/bevy_ecs_ldtk/blob/main/examples/platformer/systems.rs
///
/// Spawns heron collisions for the walls of a level
///
/// You could just insert a ColliderBundle in to the WallBundle,
/// but this spawns a different collider for EVERY wall tile.
/// This approach leads to bad performance.
///
/// Instead, by flagging the wall tiles and spawning the collisions later,
/// we can minimize the amount of colliding entities.
///
/// The algorithm used here is a nice compromise between simplicity, speed,
/// and a small number of rectangle colliders.
/// In basic terms, it will:
/// 1. consider where the walls are
/// 2. combine wall tiles into flat "plates" in each individual row
/// 3. combine the plates into rectangles across multiple rows wherever possible
/// 4. spawn colliders for each rectangle
fn spawn_wall_collision(
    mut commands: Commands,
    wall_query: Query<(&GridCoords, &Parent), Added<Block>>,
    parent_query: Query<&Parent, Without<Block>>,
    level_query: Query<(Entity, &Handle<LdtkLevel>)>,
    levels: Res<Assets<LdtkLevel>>,
) {
    /// Represents a wide wall that is 1 tile tall
    /// Used to spawn wall collisions
    #[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
    struct Plate {
        left: i32,
        right: i32,
    }

    /// A simple rectangle type representing a wall of any size
    struct Rect {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

    // Consider where the walls are
    // storing them as GridCoords in a HashSet for quick, easy lookup
    //
    // The key of this map will be the entity of the level the wall belongs to.
    // This has two consequences in the resulting collision entities:
    // 1. it forces the walls to be split along level boundaries
    // 2. it lets us easily add the collision entities as children of the appropriate level entity
    let mut level_to_wall_locations: HashMap<Entity, HashSet<GridCoords>> = HashMap::new();

    wall_query.for_each(|(&grid_coords, parent)| {
        // An intgrid tile's direct parent will be a layer entity, not the level entity
        // To get the level entity, you need the tile's grandparent.
        // This is where parent_query comes in.
        if let Ok(grandparent) = parent_query.get(parent.get()) {
            level_to_wall_locations
                .entry(grandparent.get())
                .or_default()
                .insert(grid_coords);
        }
    });

    if !wall_query.is_empty() {
        level_query.for_each(|(level_entity, level_handle)| {
            if let Some(level_walls) = level_to_wall_locations.get(&level_entity) {
                let level = levels
                    .get(level_handle)
                    .expect("Level should be loaded by this point");

                let LayerInstance {
                    c_wid: width,
                    c_hei: height,
                    grid_size,
                    ..
                } = level
                    .level
                    .layer_instances
                    .clone()
                    .expect("Level asset should have layers")[0];

                // combine wall tiles into flat "plates" in each individual row
                let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

                for y in 0..height {
                    let mut row_plates: Vec<Plate> = Vec::new();
                    let mut plate_start = None;

                    // + 1 to the width so the algorithm "terminates" plates that touch the right edge
                    for x in 0..width + 1 {
                        match (plate_start, level_walls.contains(&GridCoords { x, y })) {
                            (Some(s), false) => {
                                row_plates.push(Plate {
                                    left: s,
                                    right: x - 1,
                                });
                                plate_start = None;
                            }
                            (None, true) => plate_start = Some(x),
                            _ => (),
                        }
                    }

                    plate_stack.push(row_plates);
                }

                // combine "plates" into rectangles across multiple rows
                let mut rect_builder: HashMap<Plate, Rect> = HashMap::new();
                let mut prev_row: Vec<Plate> = Vec::new();
                let mut wall_rects: Vec<Rect> = Vec::new();

                // an extra empty row so the algorithm "finishes" the rects that touch the top edge
                plate_stack.push(Vec::new());

                for (y, current_row) in plate_stack.into_iter().enumerate() {
                    for prev_plate in &prev_row {
                        if !current_row.contains(prev_plate) {
                            // remove the finished rect so that the same plate in the future starts a new rect
                            if let Some(rect) = rect_builder.remove(prev_plate) {
                                wall_rects.push(rect);
                            }
                        }
                    }
                    for plate in &current_row {
                        rect_builder
                            .entry(plate.clone())
                            .and_modify(|e| e.top += 1)
                            .or_insert(Rect {
                                bottom: y as i32,
                                top: y as i32,
                                left: plate.left,
                                right: plate.right,
                            });
                    }
                    prev_row = current_row;
                }

                commands.entity(level_entity).with_children(|level| {
                    // Spawn colliders for every rectangle..
                    // Making the collider a child of the level serves two purposes:
                    // 1. Adjusts the transforms to be relative to the level for free
                    // 2. the colliders will be despawned automatically when levels unload
                    for wall_rect in wall_rects {
                        level
                            .spawn_empty()
                            .insert(Collider::cuboid(
                                (wall_rect.right as f32 - wall_rect.left as f32 + 1.)
                                    * grid_size as f32
                                    / 2.,
                                (wall_rect.top as f32 - wall_rect.bottom as f32 + 1.)
                                    * grid_size as f32
                                    / 2.,
                            ))
                            .insert(RigidBody::Fixed)
                            .insert(Friction::new(1.0))
                            .insert(Transform::from_xyz(
                                (wall_rect.left + wall_rect.right + 1) as f32 * grid_size as f32
                                    / 2.,
                                (wall_rect.bottom + wall_rect.top + 1) as f32 * grid_size as f32
                                    / 2.,
                                0.,
                            ))
                            .insert(GlobalTransform::default())
                            .insert(side::Trigger);
                    }
                });
            }
        });
    }
}

#[derive(Bundle, LdtkEntity)]
struct PlayerBundle {
    player: Player,
    #[from_entity_instance]
    entity_instance: EntityInstance,
    player_input: PlayerInput,
    has_side: HasSides,
    rigid_body: RigidBody,
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    velocity: Velocity,
    #[with(player_friction)]
    friction: Friction,
    #[with(entity_collider)]
    collider: Collider,
    #[with(player_mass_properties)]
    mass_properties: ColliderMassProperties,
    external_force: ExternalForce,
    external_impulse: ExternalImpulse,
    #[with(entity_name)]
    name: Name,
}

fn player_friction(_: &EntityInstance) -> Friction {
    Friction::new(1.5)
}

fn entity_collider(instance: &EntityInstance) -> Collider {
    let tile = instance.tile.as_ref().unwrap();
    Collider::cuboid(tile.w as f32 / 2.0, tile.h as f32 / 2.0)
}

fn player_mass_properties(_: &EntityInstance) -> ColliderMassProperties {
    ColliderMassProperties::Density(1.0)
}

fn entity_name(instance: &EntityInstance) -> Name {
    Name::new(instance.identifier.clone())
}

#[derive(Bundle, LdtkEntity)]
struct PowerupBundle<T: 'static + Send + Sync + Component + Default> {
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    powerup: side::Powerup,
    sensor: Sensor,
    #[with(entity_collider)]
    collider: Collider,
    effect: T,
    #[with(entity_name)]
    name: Name,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load("world.ldtk"),
            // transform: Transform::from_scale(Vec3::splat(1.0 / 8.0)),
            ..default()
        },
        Name::new("World"),
    ));

    rapier_config.gravity = Vec2::new(0.0, -240.0);
    commands.spawn({
        let mut bundle = Camera2dBundle::default();
        bundle.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(200.0);
        bundle
    });
}

fn music(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio.play_with_settings(
        asset_server.load("music.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: 0.5,
            speed: 1.0,
        },
    );
}

#[derive(Default, Component)]
pub struct PlayerInput(pub f32);

fn update_player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut inputs: Query<&mut PlayerInput, With<Player>>,
) {
    let mut dir = 0.0;
    if keyboard_input.any_pressed([KeyCode::A, KeyCode::Left]) {
        dir -= 1.0;
    }
    if keyboard_input.any_pressed([KeyCode::D, KeyCode::Right]) {
        dir += 1.0;
    }
    for mut input in inputs.iter_mut() {
        input.0 = dir;
    }
}

#[derive(Component)]
pub struct DisableRotationControl;

fn player_rotation_control(
    time: Res<Time>,
    mut query: Query<(&PlayerInput, &mut Velocity), Without<DisableRotationControl>>,
) {
    for (input, mut vel) in query.iter_mut() {
        if input.0 != 0.0 {
            let target_angvel = -input.0 * 2.0 * PI;
            let max_delta = 2.0 * PI * time.delta_seconds() * 20.0;
            vel.angvel += (target_angvel - vel.angvel).clamp(-max_delta, max_delta);
        }
    }
}

fn update_camera(
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
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
