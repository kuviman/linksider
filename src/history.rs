use super::*;

pub struct Player {
    states: Vec<GameState>,
    moves: Vec<Moves>,
    target_pos: usize,
    playback_pos: f32,
    animation_time: f32,
    auto_continue: bool,
}

pub struct Animation<'a> {
    pub prev_state: &'a GameState,
    pub moves: &'a Moves,
    pub t: f32,
}

impl Player {
    pub fn new(state: GameState, config: &logicsider::Config, animation_time: f32) -> Self {
        let mut result = Self {
            states: vec![state],
            moves: vec![],
            target_pos: 0,
            playback_pos: 0.0,
            animation_time,
            auto_continue: false,
        };
        result.process_move(config, Input::Skip);
        result
    }

    pub fn process_move(&mut self, config: &logicsider::Config, input: Input) -> Option<&Moves> {
        assert!(self.playback_pos == self.target_pos as f32);
        let mut new_state = self.states[self.target_pos].clone();
        log::debug!("Processing move (input = {input:?})...");
        let moves = new_state.process_turn(config, input);
        log::debug!("Moves = {moves:#?}");
        if let Some(moves) = moves {
            self.states.truncate(self.target_pos + 1);
            assert_eq!(self.states.len(), self.target_pos + 1);
            self.moves.truncate(self.target_pos);
            assert_eq!(self.moves.len(), self.target_pos);
            self.states.push(new_state);
            self.moves.push(moves);
            self.target_pos += 1;
            self.auto_continue = true;
            self.moves.last()
        } else {
            self.states[self.target_pos] = new_state;
            self.auto_continue = false;
            None
        }
    }
}

pub struct Frame<'a> {
    pub current_state: &'a GameState,
    pub animation: Option<Animation<'a>>,
}

impl Player {
    pub fn frame(&self) -> Frame {
        Frame {
            current_state: &self.states[self.playback_pos.ceil() as usize],
            animation: if self.target_pos as f32 == self.playback_pos {
                None
            } else {
                Some(Animation {
                    prev_state: &self.states[self.playback_pos.floor() as usize],
                    moves: &self.moves[self.playback_pos.floor() as usize],
                    t: self.playback_pos.fract(),
                })
            },
        }
    }
}

pub struct Update<'a> {
    pub started: Option<&'a Moves>,
    pub finished: Option<&'a Moves>,
}

impl Player {
    pub fn update(
        &mut self,
        delta_time: f32,
        config: &logicsider::Config,
        input: Option<Input>,
        timeline_input: Option<isize>,
    ) -> Update {
        if self.playback_pos == self.target_pos as f32 {
            if let Some(input) = timeline_input {
                self.auto_continue = false;
                match input {
                    -1 => self.undo(),
                    1 => self.redo(),
                    _ => unreachable!(),
                }
            } else {
                let mut input = input;
                if input.is_none() && self.auto_continue {
                    input = Some(Input::Skip);
                }
                if let Some(input) = input {
                    return Update {
                        started: self.process_move(config, input),
                        finished: None,
                    };
                }
            }
        } else {
            let prev_pos = self.playback_pos.floor() as usize;
            let diff = self.target_pos as f32 - self.playback_pos;
            let speed = diff.abs().max(1.0) / self.animation_time;
            let change = speed * delta_time;
            if change > diff.abs() {
                self.playback_pos = self.target_pos as f32;
            } else {
                self.playback_pos += change * diff.signum();
            }
            let new_pos = self.playback_pos.floor() as usize;
            // if auto_continue is false means we are rewinding
            if self.auto_continue && new_pos == prev_pos + 1 {
                return Update {
                    started: None,
                    finished: Some(&self.moves[prev_pos]),
                };
            }
        }
        Update {
            started: None,
            finished: None,
        }
    }

    pub fn restart(&mut self) {
        self.target_pos = 0;
    }

    pub fn undo(&mut self) {
        while self.target_pos != 0 {
            self.target_pos -= 1;
            if self.states[self.target_pos].stable {
                break;
            }
        }
    }

    pub fn redo(&mut self) {
        while self.target_pos + 1 < self.states.len() {
            self.target_pos += 1;
            if self.states[self.target_pos].stable {
                break;
            }
        }
    }

    pub fn change_player_selection(&mut self, config: &logicsider::Config, delta: isize) {
        // TODO player selection should not be part of the game state?
        self.states[self.target_pos].change_player_selection(config, delta);
    }
}
