use super::*;

#[derive(Deserialize)]
pub struct Config {
    volume: f64,
}

pub struct Sound {
    inner: geng::Sound,
}

impl geng::asset::Load for Sound {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        geng::Sound::load(manager, path)
            .map_ok(|inner| Self { inner })
            .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("wav");
}

#[derive(geng::asset::Load)]
pub struct Assets {
    #[load(ext = "mp3", postprocess = "make_looped")]
    pub music: Sound,
    pub enter_goal: Sound,
    pub magnet: Sound,
    #[load(path = "move.wav")]
    pub r#move: Sound,
    pub slide: Sound,
    pub jump: Sound,
    pub powerup: Sound,
    pub player_change: Sound,
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

fn make_looped(sound: &mut Sound) {
    sound.inner.set_looped(true);
}

pub struct State {
    assets: Rc<crate::Assets>,
    // This field used for its Drop impl
    #[allow(dead_code)]
    music: StopOnDrop,
}

impl State {
    // pgorley wants to be a part of the linksider codebase
    pub fn new(geng: &Geng, assets: &Rc<crate::Assets>) -> Self {
        geng.audio().set_volume(assets.config.sound.volume);
        Self {
            assets: assets.clone(),
            music: assets.sound.music.inner.play().stop_on_drop(),
        }
    }

    fn play(&self, sound: &Sound) {
        sound.inner.play();
    }

    pub fn play_turn_start_sounds(&self, moves: &Moves) {
        let assets = &self.assets.sound;
        for entity_move in &moves.entity_moves {
            self.play(match entity_move.move_type {
                EntityMoveType::Magnet { .. } => &assets.magnet,
                EntityMoveType::MagnetContinue => continue,
                EntityMoveType::EnterGoal { .. } => &assets.enter_goal,
                EntityMoveType::Gravity => continue,
                EntityMoveType::Move => &assets.r#move,
                EntityMoveType::Pushed => continue,
                EntityMoveType::SlideStart => &assets.slide,
                EntityMoveType::SlideContinue => continue,
                EntityMoveType::Jump { .. } => &assets.jump,
            });
        }
    }

    pub fn play_turn_end_sounds(&self, moves: &Moves) {
        let assets = &self.assets.sound;
        if !moves.collected_powerups.is_empty() {
            self.play(&assets.powerup);
        }
    }

    pub fn player_change(&self) {
        self.play(&self.assets.sound.player_change);
    }
}
