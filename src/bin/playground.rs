use bevy::{ecs::query::WorldQuery, prelude::*};

pub struct Stuff;

#[derive(Component)]
struct Position(Vec2);

#[derive(Resource)]
struct Sounds {
    hehehe: Handle<AudioSource>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: bevy::render::camera::ScalingMode::FixedVertical(10.0),
            ..default()
        },
        ..default()
    });
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..default()
            },
            transform: default(),
            texture: asset_server.load("texture.png"),
            ..default()
        },
        Position(Vec2::new(0.0, 0.0)),
    ));
    commands.insert_resource(Sounds {
        hehehe: asset_server.load("hehehe.ogg"),
    })
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct MovementQuery {
    position: &'static mut Position,
}

fn movement(
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    time: Res<Time>,
    sounds: Res<Sounds>,
    audio: Res<Audio>,
    mut query: Query<MovementQuery>,
    asset_server: Res<AssetServer>,
) {
    for mut entity in query.iter_mut() {
        if keyboard_input.any_pressed([KeyCode::Left, KeyCode::A])
            || mouse_input.any_pressed([MouseButton::Left])
        {
            entity.position.0.x -= time.delta_seconds();
        }
        if keyboard_input.any_pressed([KeyCode::Right, KeyCode::D])
            || mouse_input.any_pressed([MouseButton::Right])
        {
            entity.position.0.x += time.delta_seconds();
        }
    }

    if keyboard_input.any_just_pressed([KeyCode::Space]) {
        audio.play(sounds.hehehe.clone());
    }
}

fn update_rendering(mut query: Query<(&Position, &mut Transform)>) {
    for (position, mut transform) in query.iter_mut() {
        *transform = Transform::from_translation(position.0.extend(0.0));
    }
}

impl Plugin for Stuff {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(movement)
            .add_system(update_rendering.after(movement));
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(Stuff)
        .run();
}
