use tokio::sync::broadcast;

use crate::detection::{SessionTime, TrackState};
pub trait TrackStateMachine {
    fn handle_state(&mut self, maybe_state: Option<TrackState>, maybe_time: Option<SessionTime>);
    fn override_state(&mut self, state: TrackState);
    fn subscribe(&self) -> broadcast::Receiver<TrackState>;
}
