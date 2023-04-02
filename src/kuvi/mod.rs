use bevy::prelude::*;

pub struct Stuff;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle { ..default() });
    commands.spawn(SpriteBundle {
        transform: default(),
        texture: asset_server.load("texture.png"),
        ..default()
    });
}

impl Plugin for Stuff {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
    }
}
