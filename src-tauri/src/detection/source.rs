use crate::detection::state::{SessionTime, TrackState};

/// A DetectionSource represents any structure that can provide the current state of the track and
/// the session time.
pub trait DetectionSource {
    fn get_track_state(&mut self) -> Option<(Option<TrackState>, Option<SessionTime>)>;
}
