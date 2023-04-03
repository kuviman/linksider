use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct Plugin;

#[derive(Component)]
struct Player;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_system(update);
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
    commands.spawn((
        Player,
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(2.0, 2.0)),
                ..default()
            },
            texture: asset_server.load("texture.png"),
            ..default()
        },
        RigidBody::Dynamic,
        Velocity::zero(),
        Friction::new(1.5),
        Collider::round_cuboid(1.0, 1.0, 0.1),
        ColliderMassProperties::Density(0.1),
        ExternalForce::default(),
    ));
    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(0.0, -2.0, 0.0)),
        Collider::halfspace(Vec2::Y).unwrap(),
    ));
}

fn update(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Velocity, &mut ExternalForce), With<Player>>,
) {
    for (vel, mut force) in query.iter_mut() {
        let mut target_dir = None::<f32>;
        if keyboard_input.any_pressed([KeyCode::A, KeyCode::Left]) {
            *target_dir.get_or_insert(0.0) += 1.0;
        }
        if keyboard_input.any_pressed([KeyCode::D, KeyCode::Right]) {
            *target_dir.get_or_insert(0.0) -= 1.0;
        }
        if let Some(dir) = target_dir {
            let target_angvel = dir * 2.0 * std::f32::consts::PI;
            force.torque = (target_angvel - vel.angvel) * 10.0;
        } else {
            force.torque = 0.0;
        }
    }
}
