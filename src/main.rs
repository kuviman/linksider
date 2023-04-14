// This attr removes the console on release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*};
use bevy_ecs_ldtk::prelude::*;

mod game;

fn main() {
    let mut app = App::new();
    // This fixes sprite edges artifacts
    // https://github.com/bevyengine/bevy/issues/4748
    app.insert_resource(Msaa::Off);
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
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
                    prevent_default_event_handling: true,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()), // All textures are pixelated
    )
    .add_plugin(LdtkPlugin) // Ldtk is our level editor
    .add_plugin(game::Plugin);

    if cfg!(debug_assertions) {
        app.add_plugin(LogDiagnosticsPlugin::default())
            // .add_plugin(FrameTimeDiagnosticsPlugin::default()) // This reports FPS to console
            .add_plugin(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    }

    app.run();
}
