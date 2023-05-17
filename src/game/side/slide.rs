use crate::game::level::Blocking;

use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.register_side_effect::<Slide>("SlidePower");
        app.add_turn_system(do_slide, TurnOrder::ApplySideEffects);
        app.add_system(
            slide_move
                .before(player::move_system)
                .after(detect_side_effect::<Slide>),
        );
    }
}

#[derive(Default, Component)]
pub struct Slide;

impl SideEffect for Slide {}

fn slide_move(
    players: Query<Entity, With<Movable>>,
    mut events: EventReader<SideEffectEvent<Slide>>,
    mut commands: Commands,
) {
    for player in players.iter() {
        commands.entity(player).remove::<player::SlideMove>();
    }
    for event in events.iter() {
        if players.contains(event.player) {
            commands.entity(event.player).insert(player::SlideMove);
        }
    }
}

#[derive(Component)]
struct SlideSfx(Handle<AudioSink>);

#[allow(clippy::too_many_arguments)]
fn do_slide(
    players: Query<(&player::Input, &GridCoords, &Rotation, Option<&SlideSfx>)>,
    mut events: EventReader<SideEffectEvent<Slide>>,
    mut move_events: EventWriter<turns::MoveEvent>,
    blocked: Query<BlockedQuery, With<Blocking>>,
    audio_sinks: Res<Assets<AudioSink>>,
    mut commands: Commands,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        let Ok((player_input, player_coords, player_rotation, slide_sfx)) = players.get(event.player) else { continue };

        let next_pos = GridCoords {
            x: player_coords.x + player_input.direction.delta(),
            y: player_coords.y,
        };

        let mut stop_sfx = || {
            if let Some(sfx) = slide_sfx {
                if let Some(sink) = audio_sinks.get(&sfx.0) {
                    sink.stop();
                }
            }
            commands.entity(event.player).remove::<SlideSfx>();
        };

        if is_blocked(next_pos, &blocked) {
            if slide_sfx.is_some() {
                audio.play_sfx(asset_server.load("sfx/hitWall.wav"));
            }
            stop_sfx();
            continue;
        }

        let below = GridCoords {
            x: next_pos.x,
            y: next_pos.y - 1,
        };
        let mut next_rotation = *player_rotation;
        let mut sfx = None;
        if !is_blocked(below, &blocked) {
            next_rotation = next_rotation.rotated(player_input.direction);
            sfx = Some("sfx/slideOff.wav");
            stop_sfx();
        } else if slide_sfx.is_none() {
            let sfx = audio.play_sfx(asset_server.load("sfx/slide.wav"));
            let sfx = audio_sinks.get_handle(sfx);
            commands.entity(event.player).insert(SlideSfx(sfx));
        }

        move_events.send(turns::MoveEvent {
            player: event.player,
            coords: next_pos,
            rotation: next_rotation,
            sfx,
            end_sfx: None,
            vfx: Some(VfxBundle::new(
                *player_coords,
                0,
                "animation/slide.png",
                Some(48.0),
                false,
                false,
            )),
            end_vfx: None,
        });
    }
}
