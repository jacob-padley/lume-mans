#[derive(Debug, PartialEq, Eq, serde::Serialize, Clone, Copy)]
pub enum TrackState {
    SessionStart,
    Neutral,
    GreenFlag,
    YellowFlag,
    FullCourseYellow,
    FullCourseYellowEnding,
    SafetyCar,
    SafetyCarEnding,
    VirtualSafetyCar,
    VirtualSafetyCarEnding,
    RedFlag,
    CheckeredFlag,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct SessionTime {
    hours: i32,
    minutes: i32,
    seconds: i32,
}

impl SessionTime {
    pub fn new(hours: i32, minutes: i32, seconds: i32) -> Self {
        Self {
            hours,
            minutes,
            seconds,
        }
    }

    pub fn get_seconds(&self) -> i32 {
        self.seconds
    }

    pub fn is_zero(&self) -> bool {
        self.hours == 0 && self.minutes == 0 && self.seconds == 0
    }
}
