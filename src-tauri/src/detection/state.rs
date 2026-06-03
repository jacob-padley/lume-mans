use tauri::{AppHandle, Emitter};

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

pub struct TrackStateManager {
    state: TrackState,
}

impl TrackStateManager {
    pub fn new() -> Self {
        Self {
            state: TrackState::SessionStart,
        }
    }

    /// handle_state accepts optional detected state and session time from a DetectionSource and
    /// decides whether to mutate its internal state or not. If it decides to perform a state
    /// transition, a track-status update is emitted via the AppHandle.
    pub fn handle_state(
        &mut self,
        maybe_state: Option<TrackState>,
        maybe_time: Option<SessionTime>,
        handle: &AppHandle,
    ) {
        let mut new_state = self.state;
        if self.state == TrackState::SessionStart {
            if let Some(time) = maybe_time {
                if time.get_seconds() > 0 {
                    new_state = TrackState::GreenFlag;
                }
            }
        } else if let Some(state) = maybe_state {
            // Don't let the state change from checkered flag once it's there
            if self.state != TrackState::CheckeredFlag {
                // Check whether this state transition is allowed
                if state == TrackState::GreenFlag
                    || state == TrackState::YellowFlag
                    || state == TrackState::RedFlag
                    || state == TrackState::CheckeredFlag
                    || state == TrackState::SafetyCarEnding
                    || state == TrackState::VirtualSafetyCarEnding
                    || state == TrackState::FullCourseYellowEnding
                    || (state == TrackState::SafetyCar && self.state != TrackState::SafetyCarEnding)
                    || (state == TrackState::VirtualSafetyCar
                        && self.state != TrackState::VirtualSafetyCarEnding)
                    || (state == TrackState::FullCourseYellow
                        && self.state != TrackState::FullCourseYellowEnding)
                {
                    new_state = state;
                } else if state == TrackState::Neutral {
                    // Neutral only causes a transition under certain circumstances. A Yellow Flag
                    // can end if it disappears from the screen, but a Safety Car requires a
                    // transition to a Green Flag or Safety Car Ending first.
                    if self.state == TrackState::YellowFlag {
                        new_state = TrackState::GreenFlag;
                    }
                }
            }
        }

        // Decide if a status update is needed
        if new_state != self.state {
            self.state = new_state;
            let _ = handle.emit("track-status", &self.state);
        }
    }
}
