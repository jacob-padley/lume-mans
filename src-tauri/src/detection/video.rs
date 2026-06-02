use ocrs::{ImageSource, OcrEngine};
use std::sync::Arc;
use xcap::image::EncodableLayout;
use xcap::XCapError;

use crate::detection::source::DetectionSource;
use crate::detection::state::TrackState;

/// VideoSource represents a source of video capture that the detection system can use to look for
/// the current race status. Usually this means a system display device like a monitor. A video
/// source needs an OCR engine to perform text detection on the video image.
pub struct VideoSource {
    monitor: xcap::Monitor,
    ocr_engine: Arc<OcrEngine>,
}

impl VideoSource {
    pub fn new(option: VideoSourceOption, ocr_engine: Arc<OcrEngine>) -> anyhow::Result<Self> {
        let monitor = option.get_monitor()?;
        Ok(VideoSource {
            monitor,
            ocr_engine,
        })
    }
}

impl DetectionSource for VideoSource {
    fn get_track_state(&self) -> anyhow::Result<TrackState> {
        println!("Taking screenshot");
        let image = self.monitor.capture_image()?;
        println!("Running OCR");
        let image_source = ImageSource::from_bytes(image.as_bytes(), image.dimensions())?;
        let ocr_input = self.ocr_engine.prepare_input(image_source)?;

        println!("{}", self.ocr_engine.get_text(&ocr_input)?);
        Ok(TrackState::GreenFlag)
    }
}

/// VideoSourceOption is a serializable struct that describes an available VideoSource that could
/// be used to capture track state.
#[derive(Debug, serde::Serialize)]
pub struct VideoSourceOption {
    id: u32,
    name: String,
    is_primary: bool,
}

impl VideoSourceOption {
    /// Retrieve the list of all video input sources that can be used in detection currently.
    pub fn all() -> anyhow::Result<Vec<Self>> {
        let xcap_monitors = xcap::Monitor::all()?;
        let mut monitors: Vec<Self> = Vec::new();

        for xcap_monitor in xcap_monitors {
            let input = Self::try_from(xcap_monitor)?;
            monitors.push(input);
        }
        Ok(monitors)
    }

    pub fn primary() -> anyhow::Result<Self> {
        let xcap_monitors = xcap::Monitor::all()?;

        for xcap_monitor in xcap_monitors {
            if xcap_monitor.is_primary().unwrap_or(false) {
                return Ok(Self::try_from(xcap_monitor)?);
            }
        }

        Err(anyhow::Error::msg("No primary monitor found"))
    }

    pub fn from_id(id: u32) -> anyhow::Result<Self> {
        let xcap_monitors = xcap::Monitor::all()?;

        for xcap_monitor in xcap_monitors {
            let maybe_monitor_id = xcap_monitor.id();
            match maybe_monitor_id {
                Ok(monitor_id) => {
                    if monitor_id == id {
                        return Ok(Self::try_from(xcap_monitor)?);
                    }
                }
                Err(_) => (),
            }
        }

        Err(anyhow::Error::msg(format!(
            "No monitor found with id {}",
            id
        )))
    }

    pub fn get_monitor(&self) -> anyhow::Result<xcap::Monitor> {
        let xcap_monitors = xcap::Monitor::all()?;

        for xcap_monitor in xcap_monitors {
            let maybe_monitor_id = xcap_monitor.id();
            match maybe_monitor_id {
                Ok(monitor_id) => {
                    if monitor_id == self.id {
                        return Ok(xcap_monitor);
                    }
                }
                Err(_) => (),
            }
        }
        Err(anyhow::Error::msg("Monitor not found"))
    }
}

impl TryFrom<xcap::Monitor> for VideoSourceOption {
    type Error = XCapError;

    fn try_from(value: xcap::Monitor) -> Result<Self, Self::Error> {
        // ID is required, we allow the other fields to fail and fill them with defaults.
        let id = value.id()?;
        Ok(VideoSourceOption {
            id,
            name: value.name().unwrap_or(String::from("Unknown")),
            is_primary: value.is_primary().unwrap_or(false),
        })
    }
}
