use bevy::prelude::*;

mod daivy;
mod kuvi;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(daivy::Stuff)
        .add_plugin(kuvi::Stuff)
        .run();
}
