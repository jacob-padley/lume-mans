use crate::detection::state::{SessionTime, TrackState};
pub trait TrackStateMachine<'a> {
    fn handle_state(&mut self, maybe_state: Option<TrackState>, maybe_time: Option<SessionTime>);
    fn override_state(&mut self, state: TrackState);
    fn subscribe(&mut self, subscriber: Box<dyn Fn(TrackState, TrackState) + 'a + Send + Sync>);
}

pub struct VideoStateMachine<'a> {
    state: TrackState,
    subscribers: Vec<Box<dyn Fn(TrackState, TrackState) + 'a + Send + Sync>>,
}

impl<'a> VideoStateMachine<'a> {
    pub fn new() -> Self {
        Self {
            state: TrackState::SessionStart,
            subscribers: Vec::new(),
        }
    }

    fn set_state(&mut self, new_state: TrackState) {
        if new_state != self.state {
            // Call observers
            self.subscribers
                .iter()
                .for_each(|subscriber| subscriber(self.state, new_state));
            self.state = new_state;
        }
    }
}

impl<'a> TrackStateMachine<'a> for VideoStateMachine<'a> {
    fn subscribe(&mut self, subscriber: Box<dyn Fn(TrackState, TrackState) + 'a + Send + Sync>) {
        self.subscribers.push(subscriber)
    }

    fn override_state(&mut self, state: TrackState) {
        self.set_state(state)
    }

    /// handle_state accepts optional detected state and session time from a DetectionSource and
    /// decides whether to mutate its internal state or not. If it decides to perform a state
    /// transition, a track-status update is emitted via the AppHandle.
    fn handle_state(&mut self, maybe_state: Option<TrackState>, maybe_time: Option<SessionTime>) {
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
                    if self.state == TrackState::YellowFlag
                        || self.state == TrackState::FullCourseYellowEnding
                        || self.state == TrackState::VirtualSafetyCarEnding
                        || self.state == TrackState::SafetyCarEnding
                    {
                        new_state = TrackState::GreenFlag;
                    }
                }
            }
        }

        // Decide if a status update is needed
        self.set_state(new_state)
    }
}
