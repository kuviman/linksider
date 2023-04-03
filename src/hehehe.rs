use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            |input: Res<Input<KeyCode>>, audio: Res<Audio>, asset_server: Res<AssetServer>| {
                if input.just_pressed(KeyCode::Space) {
                    audio.play(asset_server.load("hehehe.ogg"));
                }
            },
        );
    }
}
