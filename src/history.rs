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

pub struct Frame<'a> {
    pub current_state: &'a GameState,
    pub animation: Option<Animation<'a>>,
}

impl Player {
    pub fn new(state: GameState, animation_time: f32) -> Self {
        let mut result = Self {
            states: vec![state],
            moves: vec![],
            target_pos: 0,
            playback_pos: 0.0,
            animation_time,
            auto_continue: false,
        };
        result.process_move(Input::Skip);
        result
    }

    pub fn process_move(&mut self, input: Input) {
        assert!(self.playback_pos == self.target_pos as f32);
        let mut new_state = self.states[self.target_pos].clone();
        log::debug!("Processing move (input = {input:?})...");
        let moves = new_state.process_turn(input);
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
        } else {
            self.auto_continue = false;
        }
    }

    // TODO: remove
    pub fn level(&self) -> &Rc<ldtk::Level> {
        &self.states[0].level
    }

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

    pub fn update(&mut self, delta_time: f32, input: Option<Input>, timeline_input: Option<isize>) {
        if self.playback_pos == self.target_pos as f32 {
            if let Some(input) = timeline_input {
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
                    self.process_move(input);
                }
            }
        } else {
            let diff = self.target_pos as f32 - self.playback_pos;
            let speed = diff.abs().max(1.0) / self.animation_time;
            let change = speed * delta_time;
            if change > diff.abs() {
                self.playback_pos = self.target_pos as f32;
            } else {
                self.playback_pos += change * diff.signum();
            }
        }
    }

    pub fn restart(&mut self) {
        self.target_pos = 0;
    }

    pub fn undo(&mut self) {
        if self.target_pos != 0 {
            self.target_pos -= 1;
        }
    }

    pub fn redo(&mut self) {
        if self.target_pos + 1 < self.states.len() {
            self.target_pos += 1;
        }
    }

    pub fn change_player_selection(&mut self, delta: isize) {
        // TODO figure out a better way
        self.states[self.target_pos].change_player_selection(delta);
    }
}
