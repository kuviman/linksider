use bevy::prelude::*;

mod daivy;
mod kuvi;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Jam 3 🦀".into(),
                mode: if cfg!(debug_assertions) {
                    bevy::window::WindowMode::Windowed
                } else {
                    bevy::window::WindowMode::BorderlessFullscreen
                },
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(daivy::Stuff)
        .add_plugin(kuvi::Stuff)
        .run();
}
