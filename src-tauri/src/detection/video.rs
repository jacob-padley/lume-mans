use ocrs::{ImageSource, OcrEngine};
use regex::Regex;
use std::sync::Arc;
use xcap::image::{DynamicImage, EncodableLayout};
use xcap::XCapError;

use crate::detection::source::DetectionSource;
use crate::detection::state::TrackState;

struct AbsoluteBoundingBox {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct RelativeBoundingBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

// Bounding boxes for WEC broadcast graphics as of 2026
const STATUS_BOUNDING_BOX: RelativeBoundingBox = RelativeBoundingBox {
    x: 0.049,
    y: 0.153,
    width: 0.156,
    height: 0.032,
};

const TIMER_BOUNDING_BOX: RelativeBoundingBox = RelativeBoundingBox {
    x: 0.052,
    y: 0.111,
    width: 0.143,
    height: 0.032,
};

const NOTIFICATION_BOUNDING_BOX: RelativeBoundingBox = RelativeBoundingBox {
    x: 0.430,
    y: 0.069,
    width: 0.271,
    height: 0.037,
};

impl AbsoluteBoundingBox {
    fn from_relative(rel_box: RelativeBoundingBox, screen_width: u32, screen_height: u32) -> Self {
        AbsoluteBoundingBox {
            x: ((screen_width as f32) * rel_box.x).round().abs() as u32,
            y: ((screen_height as f32) * rel_box.y).round().abs() as u32,
            width: ((screen_width as f32) * rel_box.width).round().abs() as u32,
            height: ((screen_height as f32) * rel_box.height).round().abs() as u32,
        }
    }
}

/// VideoSource represents a source of video capture that the detection system can use to look for
/// the current race status. Usually this means a system display device like a monitor. A video
/// source needs an OCR engine to perform text detection on the video image.
pub struct VideoSource {
    monitor: xcap::Monitor,
    ocr_engine: Arc<OcrEngine>,
    bounding_boxes: VideoDetectionBounds,
    detection_patterns: VideoDetectionPatterns,
}

struct VideoDetectionBounds {
    status_box: AbsoluteBoundingBox,
    timer_box: AbsoluteBoundingBox,
    notification_box: AbsoluteBoundingBox,
}

/// Regexes used in detection of text in the video. Sadly these can't be declared as constants
/// because Rust moment.
struct VideoDetectionPatterns {
    session_start: Regex,
    green_flag: Regex,
    yellow_flag: Regex,
    safety_car: Regex,
    virtual_safety_car: Regex,
    safety_car_ending: Regex,
    checkered_flag: Regex,
    full_course_yellow: Regex,
    red_flag: Regex,
}

impl VideoSource {
    pub fn new(option: VideoSourceOption, ocr_engine: Arc<OcrEngine>) -> anyhow::Result<Self> {
        let monitor = option.get_monitor()?;
        let width = monitor.width()?;
        let height = monitor.height()?;

        Ok(VideoSource {
            monitor,
            ocr_engine,
            bounding_boxes: VideoDetectionBounds {
                status_box: AbsoluteBoundingBox::from_relative(STATUS_BOUNDING_BOX, width, height),
                timer_box: AbsoluteBoundingBox::from_relative(TIMER_BOUNDING_BOX, width, height),
                notification_box: AbsoluteBoundingBox::from_relative(
                    NOTIFICATION_BOUNDING_BOX,
                    width,
                    height,
                ),
            },
            detection_patterns: VideoDetectionPatterns {
                session_start: Regex::new(r"\d+:\d+:(\d+)").unwrap(),
                green_flag: Regex::new(r"GREEN\W+FLAG").unwrap(),
                yellow_flag: Regex::new(r"YELLOW\W+FLAG").unwrap(),
                safety_car: Regex::new(r"SC|SAFETY\W+CAR").unwrap(),
                virtual_safety_car: Regex::new(r"VSC|VIRTUAL\W+SAFETY\W+CAR").unwrap(),
                safety_car_ending: Regex::new(r"ENDING|SAFETY\W+CAR\W+IN\W+THIS\W+LAP").unwrap(),
                checkered_flag: Regex::new(r"CHECKERED\W+FLAG").unwrap(),
                full_course_yellow: Regex::new(r"FCY|FULL\W+COURSE\W+YELLOW").unwrap(),
                red_flag: Regex::new(r"RED\W+FLAG").unwrap(),
            },
        })
    }
}

impl DetectionSource for VideoSource {
    fn get_track_state(&self, current_state: &TrackState) -> anyhow::Result<TrackState> {
        let status_box = &self.bounding_boxes.status_box;
        let timer_box = &self.bounding_boxes.timer_box;
        let notification_box = &self.bounding_boxes.notification_box;

        let image = DynamicImage::ImageRgba8(self.monitor.capture_image()?);

        let status_cropped = image
            .crop_imm(
                status_box.x,
                status_box.y,
                status_box.width,
                status_box.height,
            )
            .into_rgb8();
        let timer_cropped = image
            .crop_imm(timer_box.x, timer_box.y, timer_box.width, timer_box.height)
            .into_rgb8();
        let notification_cropped = image
            .crop_imm(
                notification_box.x,
                notification_box.y,
                notification_box.width,
                notification_box.height,
            )
            .into_rgb8();

        let status_source = ImageSource::from_bytes(
            status_cropped.as_bytes(),
            (status_box.width, status_box.height),
        )?;
        let timer_source = ImageSource::from_bytes(
            timer_cropped.as_bytes(),
            (timer_box.width, timer_box.height),
        )?;
        let notification_source = ImageSource::from_bytes(
            notification_cropped.as_bytes(),
            (notification_box.width, notification_box.height),
        )?;

        let status_input = self.ocr_engine.prepare_input(status_source)?;
        let timer_input = self.ocr_engine.prepare_input(timer_source)?;
        let notification_input = self.ocr_engine.prepare_input(notification_source)?;

        if *current_state == TrackState::SessionStart {
            // See if the timer has begun to tick down
            let timer_text = self.ocr_engine.get_text(&timer_input)?;
            let caps = self.detection_patterns.session_start.captures(&timer_text);
            if let Some(groups) = caps {
                let last_digits = groups.get(1);
                if let Some(seconds) = last_digits {
                    let timer_seconds = seconds.as_str().parse::<i32>().unwrap_or(0);
                    if timer_seconds > 0 {
                        // Race has started
                        return Ok(TrackState::GreenFlag);
                    }
                }
            }
        } else if *current_state == TrackState::SafetyCar {
            // See if safety car is in this lap
            let notification_text = self.ocr_engine.get_text(&notification_input)?;
            if self
                .detection_patterns
                .safety_car_ending
                .is_match(&notification_text)
            {
                return Ok(TrackState::SafetyCarEnding);
            }
        } else {
            // Look for all other possible states
            let status_text = self.ocr_engine.get_text(&status_input)?.to_uppercase();

            if self.detection_patterns.green_flag.is_match(&status_text) {
                return Ok(TrackState::GreenFlag);
            } else if self.detection_patterns.yellow_flag.is_match(&status_text) {
                return Ok(TrackState::YellowFlag);
            } else if self
                .detection_patterns
                .full_course_yellow
                .is_match(&status_text)
            {
                return Ok(TrackState::FullCourseYellow);
            } else if *current_state != TrackState::SafetyCarEnding {
                if self.detection_patterns.safety_car.is_match(&status_text) {
                    return Ok(TrackState::SafetyCar);
                } else if self
                    .detection_patterns
                    .virtual_safety_car
                    .is_match(&status_text)
                {
                    return Ok(TrackState::VirtualSafetyCar);
                }
            } else if self
                .detection_patterns
                .safety_car_ending
                .is_match(&status_text)
            {
                return Ok(TrackState::SafetyCarEnding);
            } else if self.detection_patterns.red_flag.is_match(&status_text) {
                return Ok(TrackState::RedFlag);
            } else if self
                .detection_patterns
                .checkered_flag
                .is_match(&status_text)
            {
                return Ok(TrackState::CheckeredFlag);
            }
        }

        Err(anyhow::Error::msg(
            "Could not determine track state from video stream",
        ))
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
            if let Ok(monitor_id) = maybe_monitor_id {
                if monitor_id == id {
                    return Ok(Self::try_from(xcap_monitor)?);
                }
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
            if let Ok(monitor_id) = maybe_monitor_id {
                if monitor_id == self.id {
                    return Ok(xcap_monitor);
                }
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
