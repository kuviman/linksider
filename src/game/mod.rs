use std::f32::consts::PI;

use bevy::{
    ecs::query::{ReadOnlyWorldQuery, WorldQuery},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_ecs_ldtk::{prelude::*, utils::grid_coords_to_translation};

mod animation;
mod audio;
mod background;
mod goal;
mod level;
mod player;
mod side;
mod turns;
mod util;
mod vfx;

use self::vfx::VfxBundle;
use audio::AudioExt as _;
use player::Player;
use turns::AppExt as _;
use util::{Direction, *}; // Need to shadow Direction from bevy prelude

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(turns::Plugin);
        app.add_plugin(side::Plugin);
        app.add_plugin(goal::Plugin);
        app.add_plugin(vfx::Plugin);
        app.add_plugin(animation::Plugin);
        app.add_plugin(player::Plugin);
        app.add_plugin(audio::Plugin);
        app.add_plugin(level::Plugin);
        app.add_plugin(background::Plugin);

        app.add_startup_system(setup);

        app.add_system(update_camera);

        app.add_system(highlight_selected_player);
        app.add_system(this_should_have_been_done_by_daivy_not_in_bevy_system);
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

fn entity_name(instance: &EntityInstance) -> Name {
    Name::new(instance.identifier.clone())
}

fn setup(mut commands: Commands) {
    commands.spawn({
        let mut bundle = Camera2dBundle::default();
        bundle.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(200.0);
        bundle
    });
}

fn update_camera(
    time: Res<Time>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
    player: Query<&GlobalTransform, With<player::SelectedPlayer>>,
) {
    let mut camera = camera.single_mut();
    let target = {
        let (sum, num) = player
            .iter()
            .fold((Vec2::ZERO, 0), |(sum, num), transform| {
                (sum + transform.translation().xy(), num + 1)
            });
        if num == 0 {
            warn!("No players??");
            return;
        }
        sum / num as f32
    };
    let current = camera.translation.xy();
    let mut delta = target - current;
    if delta.y.abs() < 8.0 {
        delta.y = 0.0;
    }
    let new = current + delta * (time.delta_seconds() * 10.0).min(1.0);
    camera.translation = new.extend(camera.translation.z);
}

/// Changes colors of players so that the selected one is highlighted
fn highlight_selected_player(
    mut query: Query<(&mut TextureAtlasSprite, Option<&player::SelectedPlayer>), With<Player>>,
) {
    for (mut sprite, selected) in query.iter_mut() {
        sprite.color = if selected.is_some() {
            Color::WHITE
        } else {
            Color::rgba(1.0, 1.0, 1.0, 0.5)
        };
    }
}

/// This makes the player to be shown in front of everything else
fn this_should_have_been_done_by_daivy_not_in_bevy_system(
    mut query: Query<&mut Transform, Added<Player>>,
) {
    for mut transform in query.iter_mut() {
        transform.translation.z += 123.45;
    }
}
