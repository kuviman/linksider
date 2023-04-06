use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::game::{
    config::Config,
    side::{self, powerup, Blank, Powerup, Side, SideActivateEvent},
    DisableRotationControl, PlayerInput,
};

pub fn init(app: &mut App) {
    app.add_system(effect)
        .add_system(powerup)
        .add_system(effect_toggle)
        .add_system(sound);
}

#[derive(Default, Component)]
pub struct Effect;

fn sound(
    need_start: Query<Entity, (With<Effect>, With<side::Active>, Without<Handle<AudioSink>>)>,
    need_stop: Query<(Entity, &Handle<AudioSink>), (With<Effect>, Without<side::Active>)>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    audio_sinks: Res<Assets<AudioSink>>,
    mut commands: Commands,
) {
    for entity in need_start.iter() {
        let source = asset_server.load("slide.ogg");
        let weak_handle = audio.play(source);
        let strong_handle = audio_sinks.get_handle(&weak_handle);
        commands.entity(entity).insert(strong_handle);
    }
    for (entity, sink) in need_stop.iter() {
        audio_sinks.get(sink).unwrap().stop();
        commands.entity(entity).remove::<Handle<AudioSink>>();
    }
}

fn effect_toggle(
    sides: Query<(&Parent, Option<&Handle<AudioSink>>), (With<Side>, With<Effect>)>,
    mut events: EventReader<SideActivateEvent>,
    mut commands: Commands,
) {
    let mut observed = Vec::new();
    for event in events.iter() {
        let Ok((parent, audio_sink)) = sides.get(event.side()) else { continue };
        observed.push(event.clone());
        // let Ok(mut parent) = parents.get_mut(side.parent) else { continue };
        match event {
            SideActivateEvent::Activated(_) => {
                commands.entity(parent.get()).insert(DisableRotationControl);
            }
            SideActivateEvent::Deactivated(_) => {
                commands
                    .entity(parent.get())
                    .remove::<DisableRotationControl>();
            }
        };
    }
    if !observed.is_empty() {
        info!("{observed:?}");
    }
}

fn effect(
    config: Res<Config>,
    time: Res<Time>,
    mut parents: Query<(Option<&PlayerInput>, &Transform, &mut Velocity)>,
    sides: Query<
        (&Parent, &Transform, &Handle<AudioSink>),
        (With<side::Active>, With<Side>, With<Effect>),
    >,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    for (parent, transform, audio_sink) in sides.iter() {
        let Ok((input, parent_transform, mut velocity)) = parents.get_mut(parent.get()) else { continue };
        let direction = (parent_transform.rotation * transform.rotation * Vec3::Y).xy();
        velocity.linvel += direction * time.delta_seconds() * config.slide_effect.stick_force;
        if let Some(input) = input {
            let move_direction = direction.rotate(Vec2::new(0.0, 1.0));
            velocity.linvel += move_direction
                * time.delta_seconds()
                * input.direction
                * config.slide_effect.move_force;

            if let Some(sink) = audio_sinks.get(audio_sink) {
                sink.set_volume(Vec2::dot(velocity.linvel, move_direction).abs().min(1.0));
            }
        }
    }
}

fn powerup(
    mut commands: Commands,
    sides: Query<&Parent, (With<Side>, With<Blank>)>,
    powerups: Query<(With<Powerup>, With<Effect>)>,
    mut events: EventReader<powerup::Event>,

    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        let Ok(parent) = sides.get(event.side) else { continue };
        if !powerups.contains(event.powerup) {
            continue;
        }
        commands.entity(event.powerup).despawn();
        commands.entity(event.side).insert(Effect).remove::<Blank>();

        commands
            .entity(event.side)
            .insert(Collider::cuboid(0.4, 0.1));
        commands.entity(parent.get()).insert(Friction::new(0.0));

        // TODO: different system?
        commands.entity(event.side).insert((
            Sprite {
                custom_size: Some(Vec2::new(1.0, 0.25)),
                ..default()
            },
            asset_server.load::<Image, _>("side_effects/slide.png"),
            Visibility::default(),
            ComputedVisibility::default(),
        ));
    }
}
