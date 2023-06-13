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
    pub restart: Vec<geng::Key>,
    pub undo: Vec<geng::Key>,
    pub redo: Vec<geng::Key>,
    pub next_player: Vec<geng::Key>,
    pub prev_player: Vec<geng::Key>,
    pub escape: Vec<geng::Key>,
    pub cheats: Option<CheatsControls>,
}

#[derive(geng::asset::Load, Deserialize)]
#[load(serde = "toml")]
pub struct Config {
    pub play: Rc<play::Config>,
    pub camera_speed: f32,
    pub animation_time: f32,
    pub render: renderer::Config,
    pub controls: Controls,
    pub cell_pixel_size: usize,
    pub border_radius_pixels: usize,
    pub sound: sound::Config,
    pub editor: editor::Config,
    pub level_select: Rc<level_select::Config>,
    pub input: Rc<input::Config>,
    pub deselected_player_color: Rgba<f32>,
    pub zzz_time: f32,
    pub happy: bool,
}
