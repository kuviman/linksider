use super::*;
use std::{f32::consts::PI, marker::PhantomData};

mod jump;
mod slide;

pub fn init(app: &mut App) {
    app.add_system(side_init);
    jump::init(app);
    slide::init(app);
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

trait SideEffect: Component + Default {
    fn texture() -> &'static str;
}

struct SideEffectEvent<T: SideEffect> {
    player: Entity,
    phantom_data: PhantomData<T>,
}

fn detect_side_effect<T: SideEffect>(
    sides: Query<&Side, With<T>>,
    players: Query<(Entity, &GridCoords, &Rotation, &Children), With<Player>>,
    cells: Query<(&GridCoords, &IntGridCell)>,
    mut events: EventWriter<SideEffectEvent<T>>,
) {
    for (player, player_coords, player_rotation, player_children) in players.iter() {
        if !player_children
            .iter()
            .flat_map(|&child| sides.get(child).ok())
            .any(|side| side.0 == player_rotation.0)
        {
            continue;
        }
        let below = GridCoords {
            x: player_coords.x,
            y: player_coords.y - 1,
        };
        let cell = cells.iter().find_map(|(coords, cell)| {
            if coords == &below {
                Some(cell.value)
            } else {
                None
            }
        });
        if cell == Some(BLOCK) {
            events.send(SideEffectEvent {
                player,
                phantom_data: PhantomData,
            });
        }
    }
}

trait AppExt {
    fn register_side_effect<T: SideEffect>(&mut self, ldtk_name: &str);
}

impl AppExt for App {
    fn register_side_effect<T: SideEffect>(&mut self, ldtk_name: &str) {
        self.register_ldtk_entity::<PowerupBundle<T>>(ldtk_name);
        self.add_system(collect_powerup::<T>);
        self.add_system(detect_side_effect::<T>);
        self.add_event::<SideEffectEvent<T>>();
    }
}

fn collect_powerup<T: SideEffect>(
    mut sides: Query<(&Side, &mut Handle<Image>), With<Blank>>,
    players: Query<(&GridCoords, &Rotation, &Children), With<Player>>,
    powerups: Query<(Entity, &GridCoords), (With<Powerup>, With<T>)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (player_coords, player_rotation, player_children) in players.iter() {
        for (powerup, powerup_coords) in powerups.iter() {
            if player_coords == powerup_coords {
                for &side in player_children {
                    if let Ok((side_data, mut side_texture)) = sides.get_mut(side) {
                        if side_data.0 == player_rotation.0 {
                            *side_texture = asset_server.load(T::texture());
                            commands.entity(powerup).despawn();
                            commands.entity(side).remove::<Blank>().insert(T::default());
                        }
                    }
                }
            }
        }
    }
}
