use super::*;

#[derive(Deserialize)]
pub struct CheatsControls {
    pub prev_level: Vec<geng::Key>,
    pub next_level: Vec<geng::Key>,
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

#[derive(Deserialize)]
pub struct Config {
    pub camera_speed: f32,
    pub animation_time: f32,
    pub controls: Controls,
}

// TODO #[load(serde)]
impl geng::asset::Load for Config {
    fn load(_manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        file::load_detect(path.to_owned()).boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("toml");
}
