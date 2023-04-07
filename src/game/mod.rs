use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::prelude::TilemapGridSize;
use bevy_rapier2d::prelude::*;

use self::{config::Config, side::HasSides};

pub mod config;
mod side;

pub struct Plugin;

impl Default for Config {
    fn default() -> Self {
        serde_json::from_str(include_str!("config.json")).unwrap()
    }
}

#[derive(Default, Component)]
struct Player;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Config>()
            .register_type::<Config>()
            .add_plugin(bevy_inspector_egui::quick::ResourceInspectorPlugin::<Config>::default())
            .add_startup_system(setup)
            .add_system(update_player_input)
            .add_system(player_rotation_control)
            .add_system(update_camera)
            .add_system(level_restart)
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
            .add_system(spawn_wall_collision)
            .register_ldtk_entity::<PlayerBundle>("Player")
            .register_ldtk_entity::<PowerupBundle<side::effects::jump::Effect>>("JumpPower")
            .register_ldtk_entity::<PowerupBundle<side::effects::slide::Effect>>("SlidePower");
        side::init(app);
    }
}

fn spawn_wall_collision(
    cells: Query<(&Parent, &GridCoords, &IntGridCell), Added<IntGridCell>>,
    layers: Query<&TilemapGridSize>,
    mut commands: Commands,
) {
    #[derive(Default)]
    struct ColliderBuilder {
        edges: HashMap<(i32, i32), HashSet<(i32, i32)>>,
    }

    let mut layer_map = HashMap::<Entity, ColliderBuilder>::new();
    for (parent, coords, cell) in cells.iter() {
        let grid_size = layers.get(parent.get()).unwrap();
        let builder = layer_map.entry(parent.get()).or_default();
        // TODO No way to get names instead of integers???
        let GridCoords { x, y } = *coords;
        let TilemapGridSize { x: w, y: h } = *grid_size;
        let (w, h) = (w as i32, h as i32);
        let (x, y) = (x * w, y * h);
        let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
        let loop_indices = match cell.value {
            1 => {
                // block
                vec![0, 1, 2, 3]
            }
            2 => {
                // slopeLT
                vec![0, 1, 2]
            }
            3 => {
                // slopeRT
                vec![0, 1, 3]
            }
            4 => {
                // slopeLB
                vec![1, 2, 3]
            }
            5 => {
                // slopeRB
                vec![0, 2, 3]
            }
            _ => {
                unreachable!()
            }
        };
        for i in 0..loop_indices.len() {
            let a = loop_indices[i];
            let b = loop_indices[(i + 1) % loop_indices.len()];
            let a = corners[a];
            let b = corners[b];
            builder.edges.entry(a).or_default().insert(b);
        }
    }

    for (layer, builder) in layer_map {
        let vertices: Vec<(i32, i32)> = builder.edges.keys().copied().collect();
        let vertex_to_index: HashMap<(i32, i32), u32> = vertices
            .iter()
            .copied()
            .enumerate()
            .map(|(index, vertex)| (vertex, index as u32))
            .collect();
        let mut edges = builder.edges;
        let mut to_remove = Vec::new();
        for (&a, out) in &edges {
            for &b in out {
                if edges.get(&b).map_or(false, |b_out| b_out.contains(&a)) {
                    to_remove.push((a, b));
                }
            }
        }
        for (a, b) in to_remove {
            assert!(edges.get_mut(&a).unwrap().remove(&b));
        }
        let indices = edges
            .into_iter()
            .flat_map(|(a, out)| {
                let a = vertex_to_index[&a];
                let vertex_to_index = &vertex_to_index;
                out.into_iter().map(move |b| {
                    let b = vertex_to_index[&b];
                    [a, b]
                })
            })
            .collect();
        let grid_size = layers.get(layer).unwrap();
        commands
            .spawn((
                TransformBundle::from_transform(Transform::from_translation(
                    -Vec3::new(grid_size.x, grid_size.y, 0.0) / 2.0,
                )),
                Collider::polyline(
                    vertices
                        .into_iter()
                        .map(|(x, y)| Vec2::new(x as f32, y as f32))
                        .collect(),
                    Some(indices),
                ),
                side::Trigger,
            ))
            .set_parent(layer);
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
        asset_server.load("game_music.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: 0.5,
            speed: 1.0,
        },
    );
}

#[derive(Default, Component)]
pub struct PlayerInput {
    pub direction: f32,
    pub deactivate: bool,
}

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
        input.direction = dir;
        input.deactivate = keyboard_input.any_pressed([KeyCode::Space, KeyCode::W, KeyCode::Up]);
    }
}

#[derive(Component)]
pub struct DisableRotationControl;

fn player_rotation_control(
    config: Res<Config>,
    time: Res<Time>,
    mut query: Query<(&PlayerInput, &mut Velocity), Without<DisableRotationControl>>,
) {
    for (input, mut vel) in query.iter_mut() {
        if input.direction != 0.0 {
            let target_angvel = -input.direction * config.player_rotation_speed.to_radians();
            let max_delta = time.delta_seconds() * config.player_rotation_accel.to_radians();
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
