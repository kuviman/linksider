use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_ecs_ldtk::prelude::*;

use self::config::Config;

pub mod config;

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
            .add_system(update_camera)
            .add_system(level_restart)
            .add_startup_system(music)
            .insert_resource(LevelSelection::Index(0))
            .insert_resource(LdtkSettings {
                set_clear_color: SetClearColor::FromLevelBackground,
                ..Default::default()
            })
            .register_ldtk_entity::<PlayerBundle>("Player");
        // .register_ldtk_entity::<PowerupBundle<side::effects::jump::Effect>>("JumpPower")
        // .register_ldtk_entity::<PowerupBundle<side::effects::slide::Effect>>("SlidePower");
    }
}

#[derive(Bundle, LdtkEntity)]
struct PlayerBundle {
    player: Player,
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
    effect: T,
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

pub enum Direction {
    Left,
    None,
    Right,
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
        input.direction = if dir == 0 {
            Direction::None
        } else if dir < 0 {
            Direction::Left
        } else {
            Direction::Right
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
