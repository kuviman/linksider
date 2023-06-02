use super::*;

#[derive(Deserialize)]
pub struct CheatsEffectControls {
    pub jump: geng::Key,
    pub delete: geng::Key,
}

#[derive(Deserialize)]
pub struct CheatsControls {
    pub prev_level: geng::Key,
    pub next_level: geng::Key,
    pub effect: CheatsEffectControls,
}

#[derive(Deserialize)]
pub struct Controls {
    pub left: Vec<geng::Key>,
    pub right: Vec<geng::Key>,
    pub skip: Vec<geng::Key>,
    pub next_player: Vec<geng::Key>,
    pub prev_player: Vec<geng::Key>,
    pub cheats: Option<CheatsControls>,
}

#[derive(geng::asset::Load, Deserialize)]
#[load(serde = "toml")]
pub struct Config {
    pub camera_speed: f32,
    pub animation_time: f32,
    pub controls: Controls,
    pub cell_pixel_size: usize,
    pub sound: sound::Config,
}
