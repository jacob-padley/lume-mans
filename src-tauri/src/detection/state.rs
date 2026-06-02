#[derive(Debug, serde::Serialize)]
pub enum TrackState {
    SessionStart,
    GreenFlag,
    YellowFlag,
    FullCourseYellow,
    SafetyCar,
    VirtualSafetyCar,
    SafetyCarEnding,
    RedFlag,
}