use ocrs::{ImageSource, OcrEngine};
use regex::Regex;
use std::sync::Arc;
use xcap::image::{DynamicImage, EncodableLayout};
use xcap::XCapError;

use crate::detection::source::DetectionSource;
use crate::detection::state::{SessionTime, TrackState};

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
    source: VideoSourceOption,
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
    timer: Regex,
    session_end: Regex,
    green_flag: Regex,
    yellow_flag: Regex,
    safety_car: Regex,
    virtual_safety_car: Regex,
    full_course_yellow: Regex,
    safety_car_ending: Regex,
    virtual_safety_car_ending: Regex,
    full_course_yellow_ending: Regex,
    red_flag: Regex,
    neutral: Regex,
}

impl VideoSource {
    pub fn new(option: VideoSourceOption, ocr_engine: Arc<OcrEngine>) -> anyhow::Result<Self> {
        let monitor = option.get_monitor()?;
        let width = monitor.width()?;
        let height = monitor.height()?;

        Ok(Self {
            source: option,
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
                timer: Regex::new(r"(\d*):{0,1}(\d+):(\d+)").unwrap(),
                session_end: Regex::new(r"\bFINISH\b").unwrap(),
                green_flag: Regex::new(r"GREEN\W+FLAG").unwrap(),
                yellow_flag: Regex::new(r"YELLOW\W+FLAG").unwrap(),
                safety_car: Regex::new(r"\bSC\b|SAFETY\W+CAR").unwrap(),
                virtual_safety_car: Regex::new(r"\bVSC\b|VIRTUAL\W+SAFETY\W+CAR").unwrap(),
                full_course_yellow: Regex::new(r"\bFCY\b|FULL\W+COURSE\W+YELLOW").unwrap(),
                safety_car_ending: Regex::new(r"\bENDING\b|SAFETY\W+CAR\W+IN\W+THIS\W+LAP")
                    .unwrap(),
                virtual_safety_car_ending: Regex::new(r"VSC\W+ENDING").unwrap(),
                full_course_yellow_ending: Regex::new(r"FCY\W+ENDING").unwrap(),
                red_flag: Regex::new(r"RED\W+FLAG").unwrap(),
                neutral: Regex::new(r"\bHYPERCAR\b|\bLMGT3\b|\bLMP2\b|\bRACE\b|BATTLE\W+FOR")
                    .unwrap(),
            },
        })
    }
}

impl DetectionSource for VideoSource {
    /// get_track_state reports the current TrackState and SessionTime that this source can
    /// determine. It is not always the case that we can detect this information, so it is all
    /// optional.
    fn get_track_state(&self) -> Option<(Option<TrackState>, Option<SessionTime>)> {
        // Retrieve the bounding boxes of the important parts of the screen
        let status_box = &self.bounding_boxes.status_box;
        let timer_box = &self.bounding_boxes.timer_box;
        let notification_box = &self.bounding_boxes.notification_box;

        // Take a screenshot of the race
        let image = DynamicImage::ImageRgba8(self.source.get_monitor().ok()?.capture_image().ok()?);

        // Crop out the relevant parts of the image
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

        // OCR pre-prep
        let status_source = ImageSource::from_bytes(
            status_cropped.as_bytes(),
            (status_box.width, status_box.height),
        )
        .ok()?;
        let timer_source = ImageSource::from_bytes(
            timer_cropped.as_bytes(),
            (timer_box.width, timer_box.height),
        )
        .ok()?;
        let notification_source = ImageSource::from_bytes(
            notification_cropped.as_bytes(),
            (notification_box.width, notification_box.height),
        )
        .ok()?;
        let status_input = self.ocr_engine.prepare_input(status_source).ok()?;
        let timer_input = self.ocr_engine.prepare_input(timer_source).ok()?;
        let notification_input = self.ocr_engine.prepare_input(notification_source).ok()?;

        // Read the session timer if it is available
        let timer_option = match self.ocr_engine.get_text(&timer_input) {
            Ok(ref timer_text)
                if let Some(caps) = self.detection_patterns.timer.captures(timer_text) =>
            {
                Some(SessionTime::new(
                    // These unwraps are safe since the regex guarantees three valid integer capturing groups
                    caps.get(0).unwrap().as_str().parse::<i32>().unwrap_or(0),
                    caps.get(1).unwrap().as_str().parse::<i32>().unwrap(),
                    caps.get(2).unwrap().as_str().parse::<i32>().unwrap(),
                ))
            }
            Ok(finish_text) if self.detection_patterns.session_end.is_match(&finish_text) => {
                Some(SessionTime::new(0, 0, 0))
            }
            _ => None,
        };

        // If the session timer is zero, the race has ended
        if let Some(timer) = &timer_option {
            if timer.is_zero() {
                return Some((Some(TrackState::CheckeredFlag), timer_option));
            }
        }

        // Extract the rest of the relevant text
        let status_text = self.ocr_engine.get_text(&status_input).ok()?.to_uppercase();
        let notification_text = self
            .ocr_engine
            .get_text(&notification_input)
            .ok()?
            .to_uppercase();

        // Detection priority order from this point on:
        //  Green Flag
        //  Safety Car Ending
        //  VSC Ending
        //  FCY Ending
        //  Safety Car
        //  VSC
        //  FCY
        //  Yellow Flag
        //  Red Flag
        //  Neutral
        let mut state_option: Option<TrackState> = None;
        if self.detection_patterns.green_flag.is_match(&status_text) {
            state_option = Some(TrackState::GreenFlag);
        } else if self
            .detection_patterns
            .safety_car_ending
            .is_match(&status_text)
            || self
                .detection_patterns
                .safety_car_ending
                .is_match(&notification_text)
        {
            state_option = Some(TrackState::SafetyCarEnding);
        } else if self
            .detection_patterns
            .virtual_safety_car_ending
            .is_match(&notification_text)
        {
            state_option = Some(TrackState::VirtualSafetyCarEnding);
        } else if self
            .detection_patterns
            .full_course_yellow_ending
            .is_match(&notification_text)
        {
            state_option = Some(TrackState::FullCourseYellowEnding);
        } else if self.detection_patterns.safety_car.is_match(&status_text) {
            state_option = Some(TrackState::SafetyCar);
        } else if self
            .detection_patterns
            .virtual_safety_car
            .is_match(&status_text)
        {
            state_option = Some(TrackState::VirtualSafetyCar);
        } else if self
            .detection_patterns
            .full_course_yellow
            .is_match(&status_text)
        {
            state_option = Some(TrackState::FullCourseYellow);
        } else if self.detection_patterns.yellow_flag.is_match(&status_text) {
            state_option = Some(TrackState::YellowFlag);
        } else if self.detection_patterns.red_flag.is_match(&status_text) {
            state_option = Some(TrackState::RedFlag);
        } else if self.detection_patterns.neutral.is_match(&status_text) {
            state_option = Some(TrackState::Neutral);
        }

        Some((state_option, timer_option))
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
        Ok(Self {
            id,
            name: value
                .friendly_name()
                .unwrap_or(String::from("Unknown Display")),
            is_primary: value.is_primary().unwrap_or(false),
        })
    }
}
