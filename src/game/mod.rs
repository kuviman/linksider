use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::f32::consts::PI;

mod side;

pub struct Plugin;

#[derive(Component)]
struct Player;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(update_player_input)
            .add_system(update_camera)
            .add_startup_system(music);
        side::init(app);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    rapier_config.gravity = Vec2::new(0.0, -30.0);
    commands.spawn({
        let mut bundle = Camera2dBundle::default();
        bundle.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(10.0);
        bundle
    });
    let map: Vec<Vec<char>> = include_str!("../level.txt")
        .lines()
        .map(|line| line.chars().collect())
        .collect();
    let w = map.iter().map(|row| row.len()).max().unwrap();
    let h = map.len();
    let map = |x: usize, y: usize| map[h - 1 - y].get(x).copied().unwrap_or(' ');
    let index = |x, y| (x + y * (w + 1)) as u32;
    let mut trimesh_indices = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for x in 0..w {
        for y in 0..h {
            match map(x, y) {
                '#' => trimesh_indices.extend([
                    [index(x, y), index(x + 1, y), index(x, y + 1)],
                    [index(x + 1, y + 1), index(x, y + 1), index(x + 1, y)],
                ]),
                'L' => trimesh_indices.push([index(x, y), index(x + 1, y), index(x + 1, y + 1)]),
                'R' => trimesh_indices.push([index(x, y), index(x + 1, y), index(x, y + 1)]),
                'S' => {
                    let player_size = 1.0;
                    let player_radius = player_size / 2.0;
                    let player = commands
                        .spawn((
                            Player,
                            SpriteBundle {
                                sprite: Sprite {
                                    custom_size: Some(Vec2::splat(player_size)),
                                    ..default()
                                },
                                transform: {
                                    Transform::from_xyz(x as f32 + 0.5, y as f32 + 0.5, 0.0)
                                        .with_scale(Vec3::splat(1.0))
                                },
                                texture: asset_server.load("player.png"),
                                ..default()
                            },
                            RigidBody::Dynamic,
                            Velocity::zero(),
                            Friction::new(1.5),
                            Collider::cuboid(player_radius, player_radius),
                            ColliderMassProperties::Density(1.0),
                            ExternalForce::default(),
                            ExternalImpulse::default(),
                        ))
                        .id();
                    for i in 0..4 {
                        let sensor_length = player_size * 0.01;
                        let sensor_width = player_size * 0.01;
                        commands.spawn((
                            Collider::cuboid(sensor_length / 2.0, sensor_width),
                            TransformBundle::IDENTITY,
                            side::Blank,
                            Sensor,
                            ActiveEvents::COLLISION_EVENTS,
                            ActiveCollisionTypes::all(),
                            side::Side {
                                transform: Transform::from_rotation(Quat::from_rotation_z(
                                    i as f32 * PI / 2.0,
                                ))
                                .mul_transform(
                                    Transform::from_translation(Vec3::new(0.0, player_radius, 0.0)),
                                ),
                                parent: player,
                            },
                        ));
                    }
                }
                'J' => {
                    // powerup
                    commands.spawn((
                        TransformBundle::from_transform(Transform::from_xyz(
                            x as f32 + 0.5,
                            y as f32 + 0.5,
                            0.0,
                        )),
                        Collider::ball(0.3),
                        side::Powerup,
                        Sensor,
                        side::effects::jump::Effect,
                    ));
                }
                ' ' => {}
                _ => unreachable!(),
            }
        }
    }
    commands.spawn((
        Collider::trimesh(
            (0..h + 1)
                .flat_map(|y| (0..w + 1).map(move |x| Vec2::new(x as _, y as _)))
                .collect(),
            trimesh_indices,
        ),
        side::Trigger,
    ));
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

fn update_player_input(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    for mut vel in query.iter_mut() {
        let mut target_dir = None::<f32>;
        if keyboard_input.any_pressed([KeyCode::A, KeyCode::Left]) {
            *target_dir.get_or_insert(0.0) += 1.0;
        }
        if keyboard_input.any_pressed([KeyCode::D, KeyCode::Right]) {
            *target_dir.get_or_insert(0.0) -= 1.0;
        }
        if let Some(dir) = target_dir {
            let target_angvel = dir * 2.0 * PI;
            let max_delta = 2.0 * PI * time.elapsed_seconds();
            vel.angvel += (target_angvel - vel.angvel).clamp(-max_delta, max_delta);
        }
    }
}

fn update_camera(
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Query<&Transform, With<Player>>,
) {
    let mut camera = camera.single_mut();
    camera.translation = {
        let (sum, num) = player
            .iter()
            .fold((Vec3::ZERO, 0), |(sum, num), transform| {
                (sum + transform.translation, num + 1)
            });
        sum / num as f32
    };
}
