use crate::detection::state::TrackState;

pub trait DetectionSource {
    fn get_track_state(&self) -> anyhow::Result<TrackState>;
}
