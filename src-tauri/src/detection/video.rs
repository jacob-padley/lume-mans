use serde::ser::{Serialize, Serializer, SerializeStruct};
use xcap::{XCapError};

/// VideoInput represents a source of video capture that the detection system can use to look for
/// the current race status. Usually this means a system display device like a monitor.
#[derive(Debug)]
pub struct VideoInput {
    id: u32,
    name: String,
    is_primary: bool,
}

impl VideoInput {
    /// Retrieve the list of all video input sources that can be used in detection currently.
    pub fn all() -> anyhow::Result<Vec<Self>> {
        let xcap_monitors = xcap::Monitor::all()?;
        let mut monitors: Vec<Self> = Vec::new();

        for xcap_monitor in xcap_monitors {
            let input = VideoInput::try_from(xcap_monitor)?;
            monitors.push(input);
        }
        Ok(monitors)
    }
}

impl Serialize for VideoInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut state = serializer.serialize_struct("VideoInput", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("isPrimary", &self.is_primary)?;
        state.end()
    }
}

impl TryFrom<xcap::Monitor> for VideoInput {
    type Error = XCapError;

    fn try_from(value: xcap::Monitor) -> anyhow::Result<Self, Self::Error> {
        // ID is required, we allow the other fields to fail and fill them with defaults.
        let id = value.id()?;
        Ok(VideoInput {
            id,
            name: value.name().unwrap_or(String::from("Unknown")),
            is_primary: value.is_primary().unwrap_or(false),
        })
    }
}
