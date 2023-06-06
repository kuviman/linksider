use super::*;

pub mod level;
pub mod world;

#[derive(Deserialize)]
pub struct Config {
    pub level: Rc<level::Config>,
    pub world: Rc<world::Config>,
}
