use super::*;

pub fn vec_to_rot(v: IVec2) -> i32 {
    if v.y < 0 {
        return 0;
    }
    if v.y > 0 {
        return 2;
    }
    if v.x > 0 {
        return 1;
    }
    if v.x < 0 {
        return 0;
    }
    unreachable!()
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
