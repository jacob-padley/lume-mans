use image::{EncodableLayout, ImageBuffer, Luma};
use ocrs::{ImageSource, OcrEngine};
use rten_imageproc::{min_area_rect, PointF};

pub struct OptimizedOCRFrame {
    last_frame: Option<ImageBuffer<Luma<u8>, Vec<u8>>>,
    last_text: Option<String>,
    frame_delta_threshold: f64,
    mode: OCRMode,
}

pub enum OCRMode {
    SingleWord,
    MultiWord,
}

impl OptimizedOCRFrame {
    pub fn new(mode: OCRMode, frame_delta_threshold: f64) -> Self {
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
