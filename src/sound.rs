use super::*;

#[derive(Deserialize)]
pub struct Config {
    volume: f64,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    #[load(ext = "mp3", postprocess = "make_looped")]
    music: geng::Sound,
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
    geng: Geng,
    assets: Rc<crate::Assets>,
    music: StopOnDrop,
}

impl State {
    pub fn new(geng: &Geng, assets: &Rc<crate::Assets>) -> Self {
        geng.audio().set_volume(assets.config.sound.volume);
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            music: assets.sound.music.play().stop_on_drop(),
        }
    }
}
