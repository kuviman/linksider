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
    rapier_config.gravity = Vec2::ZERO;
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
        Collider::round_cuboid(1.0, 1.0, 0.1),
        ExternalForce::default(),
    ));
}

fn update(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Velocity, &mut ExternalForce), With<Player>>,
) {
    let target_angvel = {
        let mut res = 0.0;
        if keyboard_input.any_pressed([KeyCode::A, KeyCode::Left]) {
            res = 1.0;
        }
        if keyboard_input.any_pressed([KeyCode::D, KeyCode::Right]) {
            res = -1.0;
        }
        res * 2.0 * std::f32::consts::PI
    };
    for (vel, mut force) in query.iter_mut() {
        force.torque = (target_angvel - vel.angvel) * 10.0;
    }
}
