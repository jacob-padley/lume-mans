use tauri::{AppHandle, Emitter};

#[derive(Debug, PartialEq, Eq, serde::Serialize, Clone)]
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

    pub fn set_state(&mut self, state: TrackState, handle: &AppHandle) {
        if state != self.state {
            self.state = state;
            let _ = handle.emit("track-status", &self.state);
        }
    }

    pub fn get_state(&self) -> &TrackState {
        &self.state
    }
}
