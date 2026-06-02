use crate::detection::state::TrackState;

pub trait DetectionSource {
    fn get_track_state(&self, current_state: &TrackState) -> anyhow::Result<TrackState>;
}
