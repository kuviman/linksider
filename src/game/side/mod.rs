use super::{player::Movable, turns::TurnOrder, *};
use std::{f32::consts::PI, marker::PhantomData};

mod jump;
mod magnet;
mod slide;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(jump::Plugin);
        app.add_plugin(slide::Plugin);
        app.add_plugin(magnet::Plugin);

        app.add_system(side_init);

        app.register_ldtk_entity::<DevNullBundle>("DevNull");
    }
}

#[derive(Debug, Component)]
pub struct Side(i32);

#[derive(Component)]
pub struct Blank;

#[derive(Default, Component)]
pub struct Trigger;

#[derive(Default, Component)]
pub struct PickupSideEffects;

fn side_init(query: Query<Entity, Added<PickupSideEffects>>, mut commands: Commands) {
    for player in query.iter() {
        for i in 0..4 {
            commands
                .spawn((
                    Side(i),
                    Blank,
                    TransformBundle::from_transform(
                        Transform::from_rotation(Quat::from_rotation_z(-(i + 2) as f32 * PI / 2.0))
                            * Transform::from_translation(Vec3::new(
                                0.0, 16.0, // KEKW
                                0.0,
                            )),
                    ),
                    VisibilityBundle::default(),
                ))
                .set_parent(player);
        }
    }
}

#[derive(Default, Component)]
pub struct Powerup;

/// Deletes the side effect
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
    players: Query<(Entity, &GridCoords, &Rotation, &Children), With<Movable>>,
    blocked: Query<BlockedQuery, With<Trigger>>,
    mut events: EventWriter<SideEffectEvent<T>>,
) {
    info!("CHECKING FOR ACTIVATION");
    for (player, player_coords, player_rotation, player_children) in players.iter() {
        let mut sides: Vec<&Side> = player_children
            .iter()
            .flat_map(|&child| sides.get(child).ok())
            .collect();
        sides.sort_by_key(|side| -side_vec(player_rotation.0, side.0).y);
        for side in sides {
            info!("I HAVE POWER POG");
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
                info!("ACTIVATED");
                events.send(SideEffectEvent {
                    player,
                    side: side.0,
                    phantom_data: PhantomData,
                });
                if let Some((other_player, _, other_player_rot, ..)) = players
                    .iter()
                    .find(|&(_, coords, ..)| *coords == side_coords)
                {
                    events.send(SideEffectEvent {
                        player: other_player,
                        side: player_side(other_player_rot, direction),
                        phantom_data: PhantomData,
                    });
                }
            }
        }
    }
}

#[derive(Bundle, LdtkEntity)]
struct PowerupBundle<T: 'static + Send + Sync + Component + Default> {
    #[sprite_sheet_bundle]
    sprite_sheet: SpriteSheetBundle,
    #[grid_coords]
    position: GridCoords,
    effect: T,
    #[from_entity_instance]
    rotation: Rotation,
    powerup: side::Powerup,
    #[with(entity_name)]
    name: Name,
}

trait AppExt {
    fn register_side_effect<T: SideEffect>(&mut self, ldtk_name: &str);
}

impl AppExt for App {
    fn register_side_effect<T: SideEffect>(&mut self, ldtk_name: &str) {
        self.register_ldtk_entity::<PowerupBundle<T>>(ldtk_name);
        self.add_turn_system(
            collect_powerup::<T>.before(powerups_collected),
            TurnOrder::CollectPowerups,
        );
        self.add_turn_system(
            delete_side_effect::<T>.before(powerups_collected),
            TurnOrder::CollectPowerups,
        );
        self.add_turn_system(
            detect_side_effect::<T>.after(powerups_collected),
            TurnOrder::DetectSideEffect,
        );
        self.add_event::<SideEffectEvent<T>>();
    }
}

// This is here just for the sake of ordering
fn powerups_collected() {}

fn delete_side_effect<T: SideEffect>(
    mut sides: Query<&Side, With<T>>,
    players: Query<(&GridCoords, &Rotation, &Children), With<Movable>>,
    devnulls: Query<(Entity, &GridCoords), With<DevNull>>,
    mut commands: Commands,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    for (player_coords, player_rotation, player_children) in players.iter() {
        for (devnull, devnull_coords) in devnulls.iter() {
            if player_coords == devnull_coords {
                for &side in player_children {
                    if let Ok(side_data) = sides.get_mut(side) {
                        if side_data.0 == player_rotation.0 {
                            commands.entity(devnull).despawn();
                            commands.entity(side).insert(Blank).remove::<T>();
                            commands
                                .entity(side)
                                .remove::<TextureAtlasSprite>()
                                .remove::<Handle<TextureAtlas>>();

                            audio.play_sfx(asset_server.load("sfx/hitHurt.wav"));
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn collect_powerup<T: SideEffect>(
    mut sides: Query<&Side, With<Blank>>,
    players: Query<(&GridCoords, &Rotation, &Children), With<PickupSideEffects>>,
    powerups: Query<
        (
            Entity,
            &GridCoords,
            &Rotation,
            &TextureAtlasSprite,
            &Handle<TextureAtlas>,
        ),
        (With<Powerup>, With<T>),
    >,
    mut commands: Commands,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    for (player_coords, player_rotation, player_children) in players.iter() {
        for (powerup, powerup_coords, powerup_rotation, sprite, atlas) in powerups.iter() {
            if player_coords == powerup_coords {
                for &side in player_children {
                    if let Ok(side_data) = sides.get_mut(side) {
                        if (side_data.0 - player_rotation.0 + powerup_rotation.0) % 4 == 0 {
                            commands.entity(powerup).despawn();
                            commands.entity(side).remove::<Blank>().insert(T::default());

                            commands
                                .entity(side)
                                .insert(sprite.clone())
                                .insert(atlas.clone());

                            audio.play_sfx(asset_server.load("sfx/powerUp.wav"));

                            info!("COLLECTED");
                        }
                    }
                }
            }
        }
    }
}
