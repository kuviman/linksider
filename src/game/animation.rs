use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AnimationEndSfx(None))
            .insert_resource(AnimationEndVfx(None))
            .add_system(stop_animation.in_schedule(OnExit(turns::State::Animation)))
            .add_system(start_animation.in_schedule(OnEnter(turns::State::Animation)));
    }
}

/// When animation stops, check if we need to spawn some sound/visual effects
fn stop_animation(
    mut end_sfx: ResMut<AnimationEndSfx>,
    mut end_vfx: ResMut<AnimationEndVfx>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    if let Some(source) = end_sfx.0.take() {
        audio.play_sfx(source);
    }
    if let Some(vfx) = end_vfx.0.take() {
        commands.spawn(vfx);
    }
}

#[derive(Resource)]
struct AnimationEndSfx(Option<Handle<AudioSource>>);

#[derive(Resource)]
struct AnimationEndVfx(Option<VfxBundle>);

fn start_animation(
    mut coords: Query<(&mut GridCoords, &mut Rotation)>,
    mut events: EventReader<MoveEvent>,
    mut commands: Commands,
    mut end_sfx: ResMut<AnimationEndSfx>,
    mut end_vfx: ResMut<AnimationEndVfx>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    info!("Animation started");
    let mut sfx = None;
    let mut animation_time = 0.2;
    if let Some(event) = events.iter().last() {
        if let Ok((mut coords, mut rot)) = coords.get_mut(event.player) {
            animation_time *= ((rot.0 - event.rotation.0).abs() as f32).max(1.0);
            *coords = event.coords;
            *rot = event.rotation;
            sfx = event.sfx;
            end_sfx.0 = event.end_sfx.map(|path| asset_server.load(path));
            if let Some(vfx) = event.vfx.clone() {
                let entity = commands.spawn(vfx).id();
                info!("Spawn {entity:?}");
            }
            end_vfx.0 = event.end_vfx.clone();
        }
    }
    if let Some(sfx) = sfx {
        audio.play_sfx(asset_server.load(sfx));
    }
    commands.insert_resource(TurnAnimationTimer(Timer::from_seconds(
        animation_time,
        TimerMode::Once,
    )));
}
