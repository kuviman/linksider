use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_ecs_ldtk::{GridCoords, IntGridCell};

use super::{end_turn, GameState, MoveEvent, Player, PlayerInput, Rotation, BLOCK};

pub fn init(app: &mut App) {
    app.add_system(side_init);
    app.add_systems(
        (collect_jump, detect_jump, do_jump)
            .in_set(OnUpdate(GameState::Turn))
            .before(end_turn),
    );
    app.add_event::<JumpEvent>();
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
pub struct Jump;

#[derive(Default, Component)]
pub struct Powerup;

fn collect_jump(
    mut sides: Query<(&Side, &mut Handle<Image>), With<Blank>>,
    players: Query<(&GridCoords, &Rotation, &Children), With<Player>>,
    powerups: Query<(Entity, &GridCoords), (With<Powerup>, With<Jump>)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (player_coords, player_rotation, player_children) in players.iter() {
        for (powerup, powerup_coords) in powerups.iter() {
            if player_coords == powerup_coords {
                for &side in player_children {
                    if let Ok((side_data, mut side_texture)) = sides.get_mut(side) {
                        if side_data.0 == player_rotation.0 {
                            *side_texture = asset_server.load("side_effects/jump.png");
                            commands.entity(powerup).despawn();
                            commands.entity(side).remove::<Blank>().insert(Jump);
                        }
                    }
                }
            }
        }
    }
}

struct JumpEvent(Entity);

fn detect_jump(
    sides: Query<&Side, With<Jump>>,
    players: Query<(Entity, &GridCoords, &Rotation, &Children), With<Player>>,
    cells: Query<(&GridCoords, &IntGridCell)>,
    mut events: EventWriter<JumpEvent>,
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
            events.send(JumpEvent(player));
        }
    }
}

fn do_jump(
    players: Query<(&PlayerInput, &GridCoords, &Rotation)>,
    mut events: EventReader<JumpEvent>,
    mut move_events: EventWriter<MoveEvent>,
    cells: Query<(&GridCoords, &IntGridCell)>,
) {
    for event in events.iter() {
        if let Ok((player_input, player_coords, player_rotation)) = players.get(event.0) {
            let up = GridCoords {
                x: player_coords.x,
                y: player_coords.y + 1,
            };
            let mut next = GridCoords {
                x: player_coords.x + player_input.direction.delta(),
                y: player_coords.y + 1,
            };
            let cell = cells.iter().find_map(|(coords, cell)| {
                if coords == &next {
                    Some(cell.value)
                } else {
                    None
                }
            });
            if cell == Some(BLOCK) {
                next = up;
            }
            move_events.send(MoveEvent(
                event.0,
                next,
                player_rotation.rotated(player_input.direction),
            ));
        }
    }
}
