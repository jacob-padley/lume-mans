use image::{EncodableLayout, ImageBuffer, Luma};
use ocrs::{ImageSource, OcrEngine};
use rten_imageproc::{min_area_rect, PointF, RotatedRect};
use std::cmp;

pub struct OptimizedOCRFrame {
    last_frame: Option<ImageBuffer<Luma<u8>, Vec<u8>>>,
    last_text: Option<String>,
    frame_delta_threshold: f64,
}

impl OptimizedOCRFrame {
    pub fn new(frame_delta_threshold: f64) -> Self {
        Self {
            last_frame: None,
            last_text: None,
            frame_delta_threshold,
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
        let word_rects = self.detect_bounding_boxes(frame);
        if !word_rects.is_empty() {
            if let Ok(lines) = ocr_engine.recognize_text(&input, &[word_rects]) {
                // We only expect one line
                if let Some(Some(line)) = lines.first() {
                    return Ok(line.to_string());
                }
            }
        }

        Err(anyhow::anyhow!("OCR did not find any text in the frame"))
    }

    /// Returns the ratio of new pixels in this new frame compared with the cached frame.
    fn get_frame_delta(&self, new_frame: &ImageBuffer<Luma<u8>, Vec<u8>>) -> f64 {
        let noise_threshold: u8 = 15;
        if let Some(ref last_frame) = self.last_frame {
            let num_changed_pixels = new_frame
                .as_raw()
                .iter()
                .zip(last_frame.as_raw().iter())
                .filter(|(&curr, &old)| curr.abs_diff(old) > noise_threshold)
                .count();
            return num_changed_pixels as f64 / new_frame.len() as f64;
        }
        1.0
    }

    /// Uses vertical and horizontal projection mapping to determine the bounding boxes of words
    /// assuming a single line of text that is contrasted against its background sufficiently.
    fn detect_bounding_boxes(&self, frame: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Vec<RotatedRect> {
        let pixels = frame.as_raw();
        let width = frame.width() as usize;
        let height = frame.height() as usize;

        // TODO: tune these parameters and maybe set them per OCR application. Also make them a
        // fraction of the frame size rather than absolute values where they are measurments of a
        // number of pixels.
        // Pixel brightness value to threshold on
        let threshold = 128;
        // Maximum number of odd pixels that can be present in a line before it is classified as
        // text.
        let horizontal_max = 4;
        // Maximum number of odd pixels that can be present in a line before it is classified as
        // text.
        let vertical_max = 1;
        // Minimum number of pixels needed to constitute a space character rather than just a gap
        // between letters.
        let min_space_width = 1;
        // Amount of extra room to add around detected boxes if available.
        let padding = 2;

        // Start by detecting whether the background colour is above or below the threshold.
        let corners = [
            pixels[0],
            pixels[width - 1],
            pixels[(height - 1) * width],
            pixels[(height - 1) * width + (width - 1)],
        ];
        let num_bright_corners = corners.iter().filter(|&&p| p > threshold).count();
        let is_light_background = num_bright_corners >= 2;
        let is_text_pixel = |p: u8| -> bool {
            if is_light_background {
                p < threshold
            } else {
                p > threshold
            }
        };

        // Start by detecting the top and bottom of the line
        let mut in_text = false;
        let mut y_start = 0;
        let mut y_end = 0;
        for y in 0..height {
            let mut sum = 0;
            for x in 0..width {
                if is_text_pixel(pixels[y * width + x]) {
                    sum += 1;
                }
            }
            if in_text {
                // See if we have reached the end of the text
                if sum < horizontal_max {
                    in_text = false;
                    y_end = y;
                    break;
                }
            } else {
                // See if we have reached the start of the text
                if sum > horizontal_max {
                    in_text = true;
                    y_start = y;
                }
            }
        }
        if in_text {
            // Line reaches the bottom of the image
            y_end = height - 1;
        }

        // Now detect the individual words in the line
        in_text = false;
        let mut words: Vec<(usize, usize)> = Vec::new();
        let mut x_start = 0;
        let mut x_end;
        let mut empty_space = 0;
        for x in 0..width {
            let mut sum = 0;
            for y in y_start..=y_end {
                if is_text_pixel(pixels[y * width + x]) {
                    sum += 1;
                }
            }
            if in_text {
                // See if we have reached the end of a word
                if sum < vertical_max {
                    empty_space += 1;
                    if empty_space >= min_space_width {
                        in_text = false;
                        x_end = x;
                        words.push((x_start, x_end));
                    }
                }
            } else {
                // See if we have reached the start of a word
                if sum > vertical_max {
                    in_text = true;
                    x_start = x;
                    empty_space = 0;
                }
            }
        }
        if in_text {
            // Last word reaches the end of the image
            x_end = width - 1;
            words.push((x_start, x_end));
        }

        // Turn the captured words into RotatedRects so that they can be used in OCR. Grow each
        // rect a small amount either side to make sure we don't cut off any text accidentally.
        words
            .iter()
            .filter_map(|(x_start, x_end)| {
                min_area_rect(&[
                    PointF {
                        x: x_start.saturating_sub(padding) as f32,
                        y: y_start.saturating_sub(padding) as f32,
                    },
                    PointF {
                        x: cmp::min(x_end.saturating_add(padding), width - 1) as f32,
                        y: y_start.saturating_sub(padding) as f32,
                    },
                    PointF {
                        x: cmp::min(x_end.saturating_add(padding), width - 1) as f32,
                        y: cmp::min(y_end.saturating_add(padding), height - 1) as f32,
                    },
                    PointF {
                        x: x_start.saturating_sub(padding) as f32,
                        y: cmp::min(y_end.saturating_add(padding), height - 1) as f32,
                    },
                ])
            })
            .collect()
    }
}
