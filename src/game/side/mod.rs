use super::*;
use std::{f32::consts::PI, marker::PhantomData};

mod jump;
mod magnet;
mod slide;

pub fn init(app: &mut App) {
    app.add_system(side_init);
    jump::init(app);
    slide::init(app);
    magnet::init(app);
    app.register_ldtk_entity::<DevNullBundle>("DevNull");
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

#[derive(Default, Component)]
pub struct DevNull;

#[derive(Bundle, LdtkEntity)]
struct DevNullBundle {
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    #[grid_coords]
    position: GridCoords,
    devnull: DevNull,
    #[with(entity_name)]
    name: Name,
}

trait SideEffect: Component + Default {
    fn texture() -> &'static str;
    fn active_below() -> bool {
        true
    }
    fn active_side() -> bool {
        false
    }
    fn active_above() -> bool {
        false
    }
}

#[derive(Debug)]
struct SideEffectEvent<T: SideEffect> {
    player: Entity,
    side: i32,
    phantom_data: PhantomData<T>,
}

fn detect_side_effect<T: SideEffect>(
    sides: Query<&Side, With<T>>,
    players: Query<(Entity, &GridCoords, &Rotation, &Children), With<Player>>,
    blocked: Query<BlockedQuery>,
    mut events: EventWriter<SideEffectEvent<T>>,
) {
    for (player, player_coords, player_rotation, player_children) in players.iter() {
        let mut sides: Vec<&Side> = player_children
            .iter()
            .flat_map(|&child| sides.get(child).ok())
            .collect();
        sides.sort_by_key(|side| -side_vec(player_rotation.0, side.0).y);
        for side in sides {
            if !match side_vec(player_rotation.0, side.0) {
                IVec2 { y: -1, .. } => T::active_below(),
                IVec2 { y: 1, .. } => T::active_above(),
                IVec2 { y: 0, .. } => T::active_side(),
                _ => unreachable!(),
            } {
                continue;
            }
            let direction = side_vec(player_rotation.0, side.0);
            let side_coords = (IVec2::from(*player_coords) + direction).into();
            if is_blocked(side_coords, &blocked) {
                events.send(SideEffectEvent {
                    player,
                    side: side.0,
                    phantom_data: PhantomData,
                });
            }
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
        self.add_system(delete_side_effect::<T>);
        self.add_system(detect_side_effect::<T>);
        self.add_event::<SideEffectEvent<T>>();
    }
}
fn delete_side_effect<T: SideEffect>(
    mut sides: Query<(&Side, &mut Handle<Image>), With<T>>,
    players: Query<(&GridCoords, &Rotation, &Children), With<Player>>,
    devnulls: Query<(Entity, &GridCoords), With<DevNull>>,
    mut commands: Commands,
) {
    for (player_coords, player_rotation, player_children) in players.iter() {
        for (devnull, devnull_coords) in devnulls.iter() {
            if player_coords == devnull_coords {
                for &side in player_children {
                    if let Ok((side_data, mut side_texture)) = sides.get_mut(side) {
                        if side_data.0 == player_rotation.0 {
                            *side_texture = default();
                            commands.entity(devnull).despawn();
                            commands.entity(side).insert(Blank).remove::<T>();
                        }
                    }
                }
            }
        }
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
