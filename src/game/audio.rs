use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(music);
    }
}

// AudioExt was made so that all sound effects have same volume
pub trait AudioExt {
    fn play_sfx(&self, source: Handle<AudioSource>) -> Handle<AudioSink>;
}

impl AudioExt for Audio {
    fn play_sfx(&self, source: Handle<AudioSource>) -> Handle<AudioSink> {
        self.play_with_settings(
            source,
            PlaybackSettings {
                volume: 0.1, // The volume of all sfx
                ..default()
            },
        )
    }
}

fn music(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio.play_with_settings(
        // asset_server.load("game_music.ogg"),
        asset_server.load("KuviBevy.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: 0.3,
            speed: 1.0,
        },
    );
}
