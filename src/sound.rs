use super::*;

#[derive(Deserialize)]
pub struct Config {
    volume: f64,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    #[load(ext = "mp3", postprocess = "make_looped")]
    music: geng::Sound,
    enter_goal: Rc<geng::Sound>,
    magnet: Rc<geng::Sound>,
    #[load(path = "move.wav")]
    r#move: Rc<geng::Sound>,
    slide: Rc<geng::Sound>,
    jump: Rc<geng::Sound>,
    happy: Rc<geng::Sound>,
    powerup: Rc<geng::Sound>,
    player_change: Rc<geng::Sound>,
    hit_wall: Rc<geng::Sound>,
}

struct StopOnDrop(geng::SoundEffect);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.stop();
    }
}

trait SoundEffectExt {
    fn stop_on_drop(self) -> StopOnDrop;
}

impl SoundEffectExt for geng::SoundEffect {
    fn stop_on_drop(self) -> StopOnDrop {
        StopOnDrop(self)
    }
}

fn make_looped(sound: &mut geng::Sound) {
    sound.set_looped(true);
}

pub struct State {
    assets: Rc<crate::Assets>,
    // This field used for its Drop impl
    #[allow(dead_code)]
    music: StopOnDrop,
    future_sounds: RefCell<Vec<(f32, Rc<geng::Sound>)>>,
    time: Cell<f32>,
}

impl State {
    // pgorley wants to be a part of the linksider codebase
    pub fn new(geng: &Geng, assets: &Rc<crate::Assets>) -> Self {
        geng.audio().set_volume(assets.config.sound.volume);
        Self {
            assets: assets.clone(),
            music: assets.sound.music.play().stop_on_drop(),
            time: Cell::new(0.0),
            future_sounds: default(),
        }
    }

    // TODO change handling future sounds
    pub fn update_game_tick_time(&self, delta_time: f32) {
        self.time.set(self.time.get() + delta_time);
        for (t, sound) in self.future_sounds.borrow_mut().iter_mut() {
            *t -= delta_time;
            if *t <= 0.0 {
                self.play(sound);
            }
        }
        self.future_sounds.borrow_mut().retain(|(t, _)| *t > 0.0);
    }

    fn play_after(&self, sound: &Rc<geng::Sound>, time: time::Duration) {
        self.future_sounds
            .borrow_mut()
            .push((time.as_secs_f64() as f32, sound.clone()));
    }

    fn play(&self, sound: &geng::Sound) {
        sound.play();
    }

    pub fn handle_game_event(&self, event: &Event) {
        let assets = &self.assets.sound;
        match event {
            Event::CollectedPowerup {
                entity,
                entity_side,
                powerup,
            } => self.play(&assets.powerup),
            Event::MoveStarted(entity_move) => self.play(match entity_move.move_type {
                EntityMoveType::Magnet { .. } => &assets.magnet,
                EntityMoveType::MagnetContinue => return,
                EntityMoveType::EnterGoal { .. } => &assets.enter_goal,
                EntityMoveType::Gravity => return,
                EntityMoveType::Move => &assets.r#move,
                EntityMoveType::Pushed => return,
                EntityMoveType::SlideStart => &assets.slide,
                EntityMoveType::SlideContinue => return,
                EntityMoveType::Jump {
                    blocked_angle,
                    cells_traveled,
                    jump_force,
                    ..
                } => {
                    if blocked_angle.is_some() {
                        self.play_after(
                            &assets.hit_wall,
                            time::Duration::from_secs_f64(
                                cells_traveled as f64 / jump_force as f64,
                            ),
                        );
                    }
                    if self.assets.config.happy {
                        &assets.happy
                    } else {
                        &assets.jump
                    }
                }
            }),
            Event::MoveEnded(entity_move) => {}
        }
    }

    pub fn player_change(&self) {
        self.play(&self.assets.sound.player_change);
    }
}
