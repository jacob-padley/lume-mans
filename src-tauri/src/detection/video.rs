use image::{DynamicImage, EncodableLayout, ImageBuffer, Luma, Rgb, Rgba};
use jcap::{
    capturer::{Capturer, Options, Resolution},
    frame::FrameType,
    frame::{Frame, VideoFrame},
    Target,
};
use ocrs::{ImageSource, OcrEngine};
use regex::Regex;
use rten_imageproc::{min_area_rect, PointF};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

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

// Minimum ratio of changed pixels to re-run OCR on a given region of the screen
const FRAME_DELTA_THRESHOLD: f64 = 0.05;

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
    source_option: VideoSourceOption,
    capture_active: Arc<AtomicBool>,
    ocr_engine: Arc<OcrEngine>,
    latest_frame: Arc<Mutex<Option<RawFrame>>>,
    status_ocr_frame: OptimizedOCRFrame,
    notification_ocr_frame: OptimizedOCRFrame,
    timer_ocr_frame: OptimizedOCRFrame,
    detection_patterns: VideoDetectionPatterns,
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

struct RawFrame {
    width: u32,
    height: u32,
    data: Vec<u8>,
    format: PixelFormat,
}

enum PixelFormat {
    Bgra,
    Rgba,
    Rgb,
    Luma,
    Xbgr,
}

impl VideoSource {
    pub fn new(
        source_option: VideoSourceOption,
        ocr_engine: Arc<OcrEngine>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            source_option,
            ocr_engine,
            capture_active: Arc::new(AtomicBool::new(false)),
            latest_frame: Arc::new(Mutex::new(None)),
            status_ocr_frame: OptimizedOCRFrame::new(OCRMode::MultiWord, FRAME_DELTA_THRESHOLD),
            notification_ocr_frame: OptimizedOCRFrame::new(OCRMode::MultiWord, FRAME_DELTA_THRESHOLD),
            timer_ocr_frame: OptimizedOCRFrame::new(OCRMode::SingleWord, FRAME_DELTA_THRESHOLD),
            detection_patterns: VideoDetectionPatterns {
                timer: Regex::new(r"(?:(\d{0,2}):)?(\d{1,2}):(\d{1,2})").unwrap(),
                session_end: Regex::new(r"\bFINISH\b").unwrap(),
                green_flag: Regex::new(r"GREEN\W+FLAG").unwrap(),
                yellow_flag: Regex::new(r"YELLOW\W+FLAG").unwrap(),
                safety_car: Regex::new(r"\bSC\b|SAFETY\W+CAR").unwrap(),
                virtual_safety_car: Regex::new(r"\bVSC\b|VIRTUAL\W+SAFETY\W+CAR").unwrap(),
                full_course_yellow: Regex::new(r"\bFCY\b|FULL\W+COURSE\W+YELLOW").unwrap(),
                safety_car_ending: Regex::new(r"\bENDING\b|SAFETY\W+CAR\W+IN\W+THIS\W+LAP")
                    .unwrap(),
                virtual_safety_car_ending: Regex::new(r"VSC\W+WILL\W+END").unwrap(),
                full_course_yellow_ending: Regex::new(r"FCY\W+WILL\W+END").unwrap(),
                red_flag: Regex::new(r"RED\W+FLAG").unwrap(),
                neutral: Regex::new(r"\bHYPERCAR\b|\bLMGT3\b|\bLMP2\b|\bRACE\b|BATTLE\W+FOR|VIRTUAL\W+ENERGY\W+TANK")
                    .unwrap(),
            },
        })
    }

    pub fn start_capture(&self) -> anyhow::Result<()> {
        if !jcap::has_permission() && !jcap::request_permission() {
            return Err(anyhow::anyhow!("Permission not granted to capture screen"));
        }
        let source = self.source_option.as_target().ok();
        let mut capturer = Capturer::build(Options {
            fps: 10,
            target: source,
            show_cursor: false,
            show_highlight: false,
            excluded_targets: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::Captured,
            crop_area: None,
            captures_audio: false,
            exclude_current_process_audio: false,
        })?;
        self.capture_active.store(true, Ordering::SeqCst);
        let capture_active = self.capture_active.clone();

        // Spin off a tokio thread to consume images from the stream and store them for the OCR
        // process to use when it needs to.
        let producer_frame_lock = self.latest_frame.clone();
        tauri::async_runtime::spawn_blocking(move || {
            capturer.start_capture();
            loop {
                if !capture_active.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(frame) = capturer.get_next_frame() {
                    let (width, height, data, format) = match frame {
                        Frame::Video(VideoFrame::BGRA(f)) => {
                            (f.width, f.height, f.data, PixelFormat::Bgra)
                        }
                        Frame::Video(VideoFrame::BGRx(f)) => {
                            (f.width, f.height, f.data, PixelFormat::Bgra)
                        }
                        Frame::Video(VideoFrame::RGBx(f)) => {
                            (f.width, f.height, f.data, PixelFormat::Rgba)
                        }
                        Frame::Video(VideoFrame::XBGR(f)) => {
                            (f.width, f.height, f.data, PixelFormat::Xbgr)
                        }
                        Frame::Video(VideoFrame::BGR0(f)) => {
                            (f.width, f.height, f.data, PixelFormat::Bgra)
                        }
                        Frame::Video(VideoFrame::YUVFrame(f)) => {
                            (f.width, f.height, f.luminance_bytes, PixelFormat::Luma)
                        }
                        Frame::Video(VideoFrame::RGB(f)) => {
                            (f.width, f.height, f.data, PixelFormat::Rgb)
                        }

                        // Ignore Audio or unexpected frames
                        Frame::Audio(_) => continue,
                    };

                    let mut latest_frame = producer_frame_lock.lock().unwrap();
                    *latest_frame = Some(RawFrame {
                        width: width as u32,
                        height: height as u32,
                        data,
                        format,
                    });
                } else {
                    break;
                }
            }
            capturer.stop_capture();
        });

        Ok(())
    }

    pub fn stop_capture(&self) {
        // Signal the tokio thread to stop capturing
        self.capture_active.store(false, Ordering::SeqCst)
    }
}

impl DetectionSource for VideoSource {
    /// get_track_state reports the current TrackState and SessionTime that this source can
    /// determine. It is not always the case that we can detect this information, so it is all
    /// optional.
    fn get_track_state(&mut self) -> Option<(Option<TrackState>, Option<SessionTime>)> {
        // Check that the capturer is open
        if !self.capture_active.load(Ordering::SeqCst) {
            return None;
        }

        // Take the latest screen capture frame
        let maybe_latest_frame = {
            let mut latest_frame_lock = self.latest_frame.lock().unwrap();
            latest_frame_lock.take()
        };

        if let Some(latest_frame) = maybe_latest_frame {
            // Make sure the pixel format is correct
            let image = match latest_frame.format {
                PixelFormat::Bgra => {
                    let mut rgba_data = vec![0; latest_frame.data.len()];
                    for (src, dst) in latest_frame
                        .data
                        .chunks_exact(4)
                        .zip(rgba_data.chunks_exact_mut(4))
                    {
                        dst[0] = src[2];
                        dst[1] = src[1];
                        dst[2] = src[0];
                        dst[3] = src[3];
                    }
                    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
                        latest_frame.width,
                        latest_frame.height,
                        rgba_data,
                    )
                    .unwrap();
                    DynamicImage::ImageRgba8(buffer)
                }
                PixelFormat::Rgba => {
                    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
                        latest_frame.width,
                        latest_frame.height,
                        latest_frame.data,
                    )
                    .unwrap();
                    DynamicImage::ImageRgba8(buffer)
                }
                PixelFormat::Rgb => {
                    let buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(
                        latest_frame.width,
                        latest_frame.height,
                        latest_frame.data,
                    )
                    .unwrap();
                    DynamicImage::ImageRgb8(buffer)
                }
                PixelFormat::Luma => {
                    let buffer = ImageBuffer::<Luma<u8>, _>::from_raw(
                        latest_frame.width,
                        latest_frame.height,
                        latest_frame.data,
                    )
                    .unwrap();
                    DynamicImage::ImageLuma8(buffer)
                }
                PixelFormat::Xbgr => {
                    let mut rgba_data = vec![0; latest_frame.data.len()];
                    for (src, dst) in latest_frame
                        .data
                        .chunks_exact(4)
                        .zip(rgba_data.chunks_exact_mut(4))
                    {
                        dst[0] = src[3];
                        dst[1] = src[2];
                        dst[2] = src[1];
                        dst[3] = 255;
                    }
                    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
                        latest_frame.width,
                        latest_frame.height,
                        rgba_data,
                    )
                    .unwrap();
                    DynamicImage::ImageRgba8(buffer)
                }
            };

            // Calculate the bounding boxes based on the frame size
            let status_box = AbsoluteBoundingBox::from_relative(
                STATUS_BOUNDING_BOX,
                image.width(),
                image.height(),
            );
            let notification_box = AbsoluteBoundingBox::from_relative(
                NOTIFICATION_BOUNDING_BOX,
                image.width(),
                image.height(),
            );
            let timer_box = AbsoluteBoundingBox::from_relative(
                TIMER_BOUNDING_BOX,
                image.width(),
                image.height(),
            );

            // Crop out the relevant parts of the image
            let status_cropped = image
                .crop_imm(
                    status_box.x,
                    status_box.y,
                    status_box.width,
                    status_box.height,
                )
                .into_luma8();
            let timer_cropped = image
                .crop_imm(timer_box.x, timer_box.y, timer_box.width, timer_box.height)
                .into_luma8();
            let notification_cropped = image
                .crop_imm(
                    notification_box.x,
                    notification_box.y,
                    notification_box.width,
                    notification_box.height,
                )
                .into_luma8();

            // Read the session timer if it is available
            let timer_option = match self
                .timer_ocr_frame
                .get_text(timer_cropped, &self.ocr_engine)
            {
                Some(ref timer_text)
                    if let Some(caps) = self.detection_patterns.timer.captures(timer_text) =>
                {
                    // There is not always an hour capturing group as it is optional
                    match caps.get(1) {
                        Some(hours) => Some(SessionTime::new(
                            // These unwraps are safe since the regex guarantees valid integer capturing groups
                            hours.as_str().parse::<i32>().unwrap_or(0),
                            caps.get(2).unwrap().as_str().parse::<i32>().unwrap(),
                            caps.get(3).unwrap().as_str().parse::<i32>().unwrap(),
                        )),
                        None => Some(SessionTime::new(
                            // In this case, there is no hour capturing group
                            0,
                            caps.get(2).unwrap().as_str().parse::<i32>().unwrap(),
                            caps.get(3).unwrap().as_str().parse::<i32>().unwrap(),
                        )),
                    }
                }
                Some(finish_text)
                    if self
                        .detection_patterns
                        .session_end
                        .is_match(&finish_text.to_uppercase()) =>
                {
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
            let status_text = self
                .status_ocr_frame
                .get_text(status_cropped, &self.ocr_engine)?
                .to_uppercase();
            let notification_text = self
                .notification_ocr_frame
                .get_text(notification_cropped, &self.ocr_engine)?
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

            return Some((state_option, timer_option));
        }
        None
    }
}

struct OptimizedOCRFrame {
    last_frame: Option<ImageBuffer<Luma<u8>, Vec<u8>>>,
    last_text: Option<String>,
    frame_delta_threshold: f64,
    mode: OCRMode,
}

enum OCRMode {
    SingleWord,
    MultiWord,
}

impl OptimizedOCRFrame {
    fn new(mode: OCRMode, frame_delta_threshold: f64) -> Self {
        Self {
            last_frame: None,
            last_text: None,
            frame_delta_threshold,
            mode,
        }
    }

    pub fn get_text(
        &mut self,
        new_frame: ImageBuffer<Luma<u8>, Vec<u8>>,
        ocr_engine: &OcrEngine,
    ) -> Option<String> {
        if self.last_frame.is_none() || self.last_text.is_none() {
            // This is the first frame or we have no cached text for this box
            let text = self.run_ocr(&new_frame, ocr_engine).ok()?;
            self.last_frame = Some(new_frame);
            self.last_text = Some(text.clone());
            return Some(text);
        } else if self.get_frame_delta(&new_frame) > self.frame_delta_threshold {
            // The frame is new
            let text = self.run_ocr(&new_frame, ocr_engine).ok()?;
            self.last_frame = Some(new_frame);
            self.last_text = Some(text.clone());
            return Some(text);
        }

        // Default to using the cached last text. We already checked that this is not none in the
        // earlier if statement so the unwrap is safe here.
        Some(self.last_text.clone().unwrap())
    }

    fn run_ocr(
        &self,
        frame: &ImageBuffer<Luma<u8>, Vec<u8>>,
        ocr_engine: &OcrEngine,
    ) -> anyhow::Result<String> {
        let source = ImageSource::from_bytes(frame.as_bytes(), (frame.width(), frame.height()))?;
        let input = ocr_engine.prepare_input(source)?;
        match self.mode {
            OCRMode::SingleWord => {
                let bounding_rect = vec![
                    PointF { x: 0.0, y: 0.0 },
                    PointF {
                        x: frame.width() as f32,
                        y: 0.0,
                    },
                    PointF {
                        x: frame.width() as f32,
                        y: frame.height() as f32,
                    },
                    PointF {
                        x: 0.0,
                        y: frame.height() as f32,
                    },
                ];
                let area_rect = min_area_rect(&bounding_rect);
                if let Some(line_rect) = area_rect {
                    if let Ok(lines) = ocr_engine.recognize_text(&input, &[vec![line_rect]]) {
                        // We only expect one line
                        if let Some(Some(line)) = lines.first() {
                            return Ok(line.to_string());
                        }
                    }
                }

                Err(anyhow::anyhow!("OCR did not find any text in the frame"))
            }
            OCRMode::MultiWord => ocr_engine.get_text(&input),
        }
    }

    fn get_frame_delta(&self, new_frame: &ImageBuffer<Luma<u8>, Vec<u8>>) -> f64 {
        let noise_threshold: u8 = 15;
        if let Some(ref last_frame) = self.last_frame {
            let changed_pixels = new_frame
                .as_raw()
                .iter()
                .zip(last_frame.as_raw().iter())
                .filter(|(&curr, &old)| curr.abs_diff(old) > noise_threshold)
                .count();
            return changed_pixels as f64 / new_frame.len() as f64;
        }
        1.0
    }
}

/// VideoSourceOption is a serializable struct that describes an available VideoSource that could
/// be used to capture track state.
#[derive(Debug, serde::Serialize)]
pub struct VideoSourceOption {
    id: u32,
    name: String,
    is_primary: bool,
    source_type: VideoSourceType,
}

#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VideoSourceType {
    Window,
    Monitor,
}

impl VideoSourceOption {
    /// Retrieve the list of all video input sources that can be used in detection currently.
    pub fn all() -> Vec<Self> {
        Vec::from_iter(jcap::get_all_targets().iter().map(Self::from))
    }

    pub fn primary() -> Self {
        Self::from(&Target::Display(jcap::get_main_display()))
    }

    pub fn get(id: u32, source_type: VideoSourceType) -> anyhow::Result<Self> {
        let targets = jcap::get_all_targets();

        for target in targets {
            match target {
                Target::Window(ref window_target) => {
                    if source_type == VideoSourceType::Window && window_target.id == id {
                        return Ok(Self::from(&target));
                    }
                }
                Target::Display(ref display_target) => {
                    if source_type == VideoSourceType::Monitor && display_target.id == id {
                        return Ok(Self::from(&target));
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "No video source found with id {} and type {:?}",
            id,
            source_type
        ))
    }

    pub fn as_target(&self) -> anyhow::Result<Target> {
        let targets = jcap::get_all_targets();

        for target in targets {
            match target {
                Target::Window(ref window_target) => {
                    if self.source_type == VideoSourceType::Window && window_target.id == self.id {
                        return Ok(target);
                    }
                }
                Target::Display(ref display_target) => {
                    if self.source_type == VideoSourceType::Monitor && display_target.id == self.id
                    {
                        return Ok(target);
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Target not found"))
    }
}

impl From<&Target> for VideoSourceOption {
    fn from(value: &Target) -> Self {
        // ID is required, we allow the other fields to fail and fill them with defaults.
        match value {
            Target::Window(window_target) => Self {
                id: window_target.id,
                name: window_target.title.clone(),
                is_primary: false,
                source_type: VideoSourceType::Window,
            },
            Target::Display(display_target) => Self {
                id: display_target.id,
                name: display_target.title.clone(),
                is_primary: display_target.id == jcap::get_main_display().id,
                source_type: VideoSourceType::Monitor,
            },
        }
    }
}
