use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

mod game;
mod hehehe;

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
        .add_plugin(hehehe::Plugin)
        .add_plugin(game::Plugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1.0))
        .add_plugin(RapierDebugRenderPlugin::default())
        .run();
}
