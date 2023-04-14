use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_system(background_tiles);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    for x in -2..=2 {
        for y in -2..=2 {
            commands.spawn((
                SpriteBundle {
                    texture: asset_server.load("parallax_bg_bottom.png"),
                    ..default()
                },
                BackgroundTile(0.75, x, y),
            ));
            commands.spawn((
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    texture: asset_server.load("parallax_bg_top.png"),
                    ..default()
                },
                BackgroundTile(0.5, x, y),
            ));
        }
    }
}

#[derive(Component)]
struct BackgroundTile(f32, i32, i32);

fn background_tiles(
    mut query: Query<(&mut Transform, &BackgroundTile), Without<Camera2d>>,
    camera: Query<&Transform, With<Camera2d>>,
) {
    let camera = camera.single();
    for (mut transform, tile) in query.iter_mut() {
        transform.translation.x = camera.translation.x * tile.0 + tile.1 as f32 * 256.0;
        transform.translation.y = camera.translation.y * tile.0 + tile.2 as f32 * 256.0;
    }
}
