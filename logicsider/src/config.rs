use super::*;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub allow_unstable_player_selection: bool,
    pub magnet: systems::magnet::Config,
    pub entities: HashMap<String, Properties>,
}
