pub enum LightingCommand {
    PlayPlayback(PlayPlaybackCommand),
    ReleaseAllPlaybacks(ReleaseAllPlaybacksCommand),
}

pub struct PlayPlaybackCommand {
    pub handle: PlaybackHandle,
    pub level: PlaybackLevel,
    pub accuracy: f32,
}

impl Default for PlayPlaybackCommand {
    fn default() -> Self {
        Self {
            handle: Default::default(),
            level: Default::default(),
            accuracy: 1.0,
        }
    }
}

impl PlayPlaybackCommand {
    pub fn from_handle(handle: PlaybackHandle) -> Self {
        Self {
            handle,
            ..Default::default()
        }
    }
}

pub enum PlaybackHandle {
    UserNumber(u32),
    #[allow(dead_code)]
    Location(String),
    #[allow(dead_code)]
    TitanId(u32),
}

impl Default for PlaybackHandle {
    fn default() -> Self {
        Self::UserNumber(1)
    }
}

pub enum PlaybackLevel {
    Level(f32),
    #[allow(dead_code)]
    LevelDelta(f32),
}

impl Default for PlaybackLevel {
    fn default() -> Self {
        Self::Level(1.0)
    }
}

pub struct ReleaseAllPlaybacksCommand {
    pub fade_time: u32,
    pub use_master_release_time: bool,
}

impl Default for ReleaseAllPlaybacksCommand {
    fn default() -> Self {
        Self {
            fade_time: 0,
            use_master_release_time: true,
        }
    }
}
