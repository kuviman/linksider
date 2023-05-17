use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_state::<State>();

        app.configure_set(TurnOrder::CollectPowerups.before(TurnOrder::ApplyBuffers1));
        app.configure_set(TurnOrder::DetectSideEffect.after(TurnOrder::ApplyBuffers1));
        app.configure_set(TurnOrder::DetectSideEffect.before(TurnOrder::ApplyBuffers2));
        app.configure_set(TurnOrder::ApplySideEffects.after(TurnOrder::ApplyBuffers2));
        app.add_system(apply_system_buffers.in_set(TurnOrder::ApplyBuffers1));
        app.add_system(apply_system_buffers.in_set(TurnOrder::ApplyBuffers2));

        app.add_system(loading_level_finish);
        app.add_systems(
            (start_turn, apply_system_buffers)
                .chain()
                .in_set(OnUpdate(State::Turn)),
        );
        app.add_system(end_turn.in_set(OnUpdate(State::Turn)));
        app.add_system(stop_animation.in_set(OnUpdate(State::Animation)));
        app.add_system(process_animation.in_set(OnUpdate(State::Animation)));

        app.add_event::<MoveEvent>();
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub enum TurnOrder {
    CollectPowerups,
    ApplyBuffers1,
    DetectSideEffect,
    ApplyBuffers2,
    ApplySideEffects,
}

pub struct MoveEvent {
    pub player: Entity,
    pub coords: GridCoords,
    pub rotation: Rotation,
    pub sfx: Option<&'static str>,
    pub end_sfx: Option<&'static str>,
    pub vfx: Option<VfxBundle>,
    pub end_vfx: Option<VfxBundle>,
}

#[derive(Resource)]
pub struct AnimationTimer(Timer);

impl AnimationTimer {
    pub fn new(animation_time_seconds: f32) -> Self {
        Self(Timer::from_seconds(animation_time_seconds, TimerMode::Once))
    }
    /// Returns value from 0 (start of animation) to 1 (end of animation)
    pub fn progress(&self) -> f32 {
        self.0.elapsed_secs() / self.0.duration().as_secs_f32()
    }
}

pub trait AppExt {
    fn add_turn_system<M>(&mut self, system: impl IntoSystemAppConfig<M>, when: TurnOrder);
}

impl AppExt for App {
    fn add_turn_system<M>(&mut self, system: impl IntoSystemAppConfig<M>, when: TurnOrder) {
        self.add_system(
            system
                .into_app_config()
                .in_set(OnUpdate(State::Turn))
                .in_set(when)
                .after(start_turn)
                .before(end_turn),
        );
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum State {
    #[default]
    LoadingLevel,
    /// This state is only happening for 1 frame, ending with the end_turn system
    Turn,
    WaitingForInput,
    Animation,
}

fn loading_level_finish(
    mut next_state: ResMut<NextState<State>>,
    query: Query<(), Added<Handle<LdtkLevel>>>,
) {
    if !query.is_empty() {
        next_state.set(State::Turn);
    }
}

/// This just advances the animation timer
fn process_animation(mut turn_timer: ResMut<AnimationTimer>, time: Res<Time>) {
    turn_timer.0.tick(time.delta()).elapsed_secs();
}

fn stop_animation(mut next_state: ResMut<NextState<State>>, turn_timer: Res<AnimationTimer>) {
    if turn_timer.0.finished() {
        info!("Animation finished");
        next_state.set(State::Turn);
    }
}

/// This is here just for the sake of ordering
fn start_turn() {}

fn end_turn(mut next_state: ResMut<NextState<State>>, events: EventReader<MoveEvent>) {
    // No events means no animation to play so we wait for player input
    if events.is_empty() {
        info!("Waiting for input now");
        next_state.set(State::WaitingForInput);
    } else {
        next_state.set(State::Animation);
    }
}
