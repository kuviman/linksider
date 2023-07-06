use super::*;

enum Mode {
    Playing(GameState),
    Rewinding {
        current_time: Time,
        target_time: Time,
    },
}

pub struct Player {
    history: Vec<GameState>,
    mode: Mode,
    animation_speed: f32,
}

impl Player {
    pub fn new(state: GameState, config: &logicsider::Config, animation_speed: f32) -> Self {
        Self {
            history: vec![],
            mode: Mode::Playing(state),
            animation_speed,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Frame<'a> {
    pub state: &'a GameState,
    pub time_since: f32,
}

impl Player {
    pub fn current_time(&self) -> Time {
        todo!()
    }

    pub fn frame(&self) -> Frame {
        match &self.mode {
            Mode::Playing(state) => Frame {
                state,
                time_since: 0.0,
            },
            Mode::Rewinding {
                current_time,
                target_time,
            } => todo!(),
        }
    }
}

impl Player {
    pub fn update(
        &mut self,
        config: &logicsider::Config,
        input: Option<Input>,
        delta_time: f32,
        mut event_handler: impl FnMut(Event),
    ) {
        let delta_time = delta_time * self.animation_speed;
        match &mut self.mode {
            Mode::Playing(state) => {
                let mut inputs = HashMap::new();
                if let Some(input) = input {
                    if let Some(entity) = state.selected_entity() {
                        inputs.insert(entity.id, input);
                    }
                }
                state.update(
                    config,
                    &inputs,
                    Time::from_secs_f32(delta_time),
                    |state| {
                        // TODO record
                    },
                    event_handler,
                );
            }
            Mode::Rewinding {
                current_time,
                target_time,
            } => {
                let diff = target_time.as_secs_f32() - current_time.as_secs_f32();
                let speed = diff.abs().max(1.0);
                let change = speed * delta_time;
                if change > diff.abs() {
                    *current_time = *target_time;
                } else {
                    *current_time = *current_time + Time::from_secs_f32(change * diff.signum());
                }
            }
        }
    }

    pub fn restart(&mut self) {
        self.mode = Mode::Rewinding {
            current_time: self.current_time(),
            target_time: Time::ZERO,
        };
    }

    pub fn undo(&mut self) {
        todo!()
    }

    pub fn redo(&mut self) {
        todo!()
    }

    pub fn change_player_selection(&mut self, config: &logicsider::Config, delta: isize) {
        todo!()
    }
}
