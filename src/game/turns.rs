use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_state::<turns::State>();

        app.add_system(loading_level_finish);
        app.add_system(end_turn.in_set(OnUpdate(turns::State::Turn)));
        app.add_system(stop_animation.in_set(OnUpdate(turns::State::Animation)));
        app.add_system(process_animation.in_set(OnUpdate(turns::State::Animation)));

        app.add_event::<MoveEvent>();
    }
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
    fn add_turn_system<M>(&mut self, system: impl IntoSystemAppConfig<M>);
}

impl AppExt for App {
    fn add_turn_system<M>(&mut self, system: impl IntoSystemAppConfig<M>) {
        self.add_system(
            system
                .into_app_config()
                .in_set(OnUpdate(State::Turn))
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
    mut next_state: ResMut<NextState<turns::State>>,
    query: Query<(), Added<Handle<LdtkLevel>>>,
) {
    if !query.is_empty() {
        next_state.set(turns::State::Turn);
    }
}

/// This just advances the animation timer
fn process_animation(mut turn_timer: ResMut<AnimationTimer>, time: Res<Time>) {
    turn_timer.0.tick(time.delta()).elapsed_secs();
}

fn stop_animation(
    mut next_state: ResMut<NextState<turns::State>>,
    turn_timer: Res<AnimationTimer>,
) {
    if turn_timer.0.finished() {
        info!("Animation finished");
        next_state.set(turns::State::Turn);
    }
}

fn end_turn(mut next_state: ResMut<NextState<turns::State>>, events: EventReader<MoveEvent>) {
    // No events means no animation to play so we wait for player input
    if events.is_empty() {
        info!("Waiting for input now");
        next_state.set(turns::State::WaitingForInput);
    } else {
        next_state.set(turns::State::Animation);
    }
}
