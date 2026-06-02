#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub enum TrackState {
    SessionStart,
    GreenFlag,
    YellowFlag,
    FullCourseYellow,
    SafetyCar,
    VirtualSafetyCar,
    SafetyCarEnding,
    RedFlag,
    CheckeredFlag,
}

pub struct TrackStateManager {
    state: TrackState,
}

impl TrackStateManager {
    pub fn new() -> Self {
        TrackStateManager {
            state: TrackState::SessionStart,
        }
    }

    pub fn set_state(&mut self, state: TrackState) {
        if state != self.state {
            println!("Track state update: {:?} -> {:?}", self.state, state);
            self.state = state;
        }
    }
}
