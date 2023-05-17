use bevy::utils::HashMap;

use super::{player::Movable, *};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AnimationEndSfx(None));
        app.insert_resource(AnimationEndVfx(None));

        app.add_system(init_prev_coords.in_schedule(OnEnter(turns::State::Turn)));

        app.add_system(start_animation.in_schedule(OnEnter(turns::State::Animation)));
        app.add_system(setup_rotation_transform);
        app.add_system(update_transforms.in_set(OnUpdate(turns::State::Animation)));
        app.add_system(stop_animation.in_schedule(OnExit(turns::State::Animation)));
    }
}

fn setup_rotation_transform(mut query: Query<(&mut Transform, &Rotation), Added<Rotation>>) {
    for (mut transform, rotation) in query.iter_mut() {
        // this is me converting the rotations into radians yea
        transform.rotation = Quat::from_rotation_z(rotation.to_radians());
    }
}

#[derive(Component)]
struct PrevCoords(GridCoords);

#[derive(Component)]
struct PrevRotation(Rotation);

fn init_prev_coords(
    query: Query<(Entity, &GridCoords, &Rotation), With<Movable>>,
    mut commands: Commands,
) {
    for (entity, position, rot) in query.iter() {
        commands
            .entity(entity)
            .insert(PrevCoords(*position))
            .insert(PrevRotation(*rot));
    }
}

#[derive(Resource)]
struct AnimationEndSfx(Option<Handle<AudioSource>>);

#[derive(Resource)]
struct AnimationEndVfx(Option<VfxBundle>);

fn start_animation(
    mut coords: Query<(&mut GridCoords, &mut Rotation)>,
    mut events: EventReader<turns::MoveEvent>,
    mut commands: Commands,
    mut end_sfx: ResMut<AnimationEndSfx>,
    mut end_vfx: ResMut<AnimationEndVfx>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    info!("Animation started");
    // TODO multiple sfx for each player?
    let mut sfx = None;
    // TODO what if multiple players have different animation time?
    let mut animation_time = 0.2;
    let mut event_per_player = HashMap::new();
    for event in events.iter() {
        event_per_player.insert(event.player, event);
    }
    for event in event_per_player.into_values() {
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
    commands.insert_resource(turns::AnimationTimer::new(animation_time));
}

fn update_transforms(
    timer: Res<turns::AnimationTimer>,
    mut query: Query<(
        &PrevCoords,
        &GridCoords,
        &PrevRotation,
        &Rotation,
        &mut Transform,
    )>,
) {
    for (prev_coords, coords, prev_rot, rot, mut transform) in query.iter_mut() {
        let t = timer.progress();

        let prev_coords = &prev_coords.0;
        let tile_size = IVec2::new(16, 16); // TODO load from ldtk
        let prev_pos = grid_coords_to_translation(*prev_coords, tile_size);
        let next_pos = grid_coords_to_translation(*coords, tile_size);
        let prev_rot = &prev_rot.0;
        let prev_rot = prev_rot.to_radians();
        let rot = rot.to_radians();
        let delta_pos = next_pos - prev_pos;
        let delta_rot = rot - prev_rot;

        if delta_rot != 0.0 {
            let rotation_origin = prev_pos
                + delta_pos / 2.0
                + Vec2::new(0.0, 1.0).rotate(delta_pos) / (delta_rot / 2.0).tan() / 2.0;

            let border_radius: f32 = delta_rot.abs() / PI * 8.0;

            let extra_len =
                (1.0 / ((1.0 - (t - 0.5).abs() * 2.0) * PI / 4.0).cos() - 1.0) * border_radius;

            *transform = Transform::from_translation(prev_pos.extend(transform.translation.z))
                .with_rotation(Quat::from_rotation_z(prev_rot));
            transform.rotate_around(
                rotation_origin.extend(123.45),
                Quat::from_rotation_z(delta_rot * t),
            );
            transform.translation = (transform.translation.xy()
                + (rotation_origin - transform.translation.xy()).normalize_or_zero() * extra_len)
                .extend(transform.translation.z);
        } else {
            let interpolated_coords = prev_pos + delta_pos * t;
            transform.translation.x = interpolated_coords.x;
            transform.translation.y = interpolated_coords.y;
        }
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
