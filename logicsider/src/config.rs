use super::*;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub magnet_continue: systems::magnet::ContinueConfig,
}
