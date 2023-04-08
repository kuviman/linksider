use super::*;
use std::f32::consts::PI;

mod jump;
pub use jump::Jump;

pub fn init(app: &mut App) {
    app.add_system(side_init);
    jump::init(app);
}

#[derive(Debug, Component)]
pub struct Side(i32);

#[derive(Component)]
pub struct Blank;

fn side_init(query: Query<Entity, Added<Player>>, mut commands: Commands) {
    for player in query.iter() {
        for i in 0..4 {
            commands
                .spawn((
                    Side(i),
                    Blank,
                    SpriteBundle {
                        transform: Transform::from_rotation(Quat::from_rotation_z(
                            -i as f32 * PI / 2.0,
                        )) * Transform::from_translation(Vec3::new(0.0, -8.0, 0.0)), // KEKW
                        ..default()
                    },
                ))
                .set_parent(player);
        }
    }
}

#[derive(Default, Component)]
pub struct Powerup;
