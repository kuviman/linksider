use super::*;

pub fn init(app: &mut App) {
    app.register_side_effect::<Slide>("SlidePower");
    app.add_system(do_slide.in_set(OnUpdate(GameState::Turn)).before(end_turn));
    app.add_system(
        slide_move
            .before(player_move)
            .after(detect_side_effect::<Slide>),
    );
}

#[derive(Default, Component)]
pub struct Slide;

impl SideEffect for Slide {}

fn slide_move(
    players: Query<Entity, With<Player>>,
    mut events: EventReader<SideEffectEvent<Slide>>,
    mut commands: Commands,
) {
    for player in players.iter() {
        commands.entity(player).remove::<SlideMove>();
    }
    for event in events.iter() {
        if players.contains(event.player) {
            commands.entity(event.player).insert(SlideMove);
        }
    }
}

#[derive(Component)]
struct SlideSfx(Handle<AudioSink>);

fn do_slide(
    players: Query<(&PlayerInput, &GridCoords, &Rotation, Option<&SlideSfx>)>,
    mut events: EventReader<SideEffectEvent<Slide>>,
    mut move_events: EventWriter<MoveEvent>,
    blocked: Query<BlockedQuery>,
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
        } else {
            if slide_sfx.is_none() {
                let sfx = audio.play_sfx(asset_server.load("sfx/slide.wav"));
                let sfx = audio_sinks.get_handle(sfx);
                commands.entity(event.player).insert(SlideSfx(sfx));
            }
            // TODO vfx
        }

        move_events.send(MoveEvent {
            player: event.player,
            coords: next_pos,
            rotation: next_rotation,
            sfx,
        });
    }
}
