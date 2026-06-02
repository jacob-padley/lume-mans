mod commands;
mod detection;

use tauri::{Manager, path::BaseDirectory};
use rten::Model;
use ocrs::{OcrEngine, OcrEngineParams};
use std::sync::{Arc, RwLock, atomic::AtomicBool};

use crate::detection::video::VideoInput;

struct AppState {
    ocr_engine: Arc<OcrEngine>,
    capture_active: Arc<AtomicBool>,
    capture_device: Arc<RwLock<VideoInput>>,
    capture_settings: RwLock<CaptureSettings>,
}

struct CaptureSettings {
    interval: u32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Workaround for crash on Wayland + Nvidia drivers
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .setup(|app| {
            let detection_model_path = app.path().resolve("resources/text-detection.rten", BaseDirectory::Resource).expect("Failed to load text detection model");
            let recognition_model_path = app.path().resolve("resources/text-recognition.rten", BaseDirectory::Resource).expect("Failed to load text recognition model");

            let detection_model = Model::load_file(detection_model_path)?;
            let recognition_model = Model::load_file(recognition_model_path)?;

            let ocr_engine = OcrEngine::new(OcrEngineParams{
                detection_model: Some(detection_model),
                recognition_model: Some(recognition_model),
                ..Default::default()
            }).expect("Failed to initialise OCR engine");

            let default_capture_device = VideoInput::primary().expect("Failed to find primary video capture device");
            
            app.manage(AppState {
                ocr_engine: Arc::new(ocr_engine),
                capture_active: Arc::new(AtomicBool::new(false)),
                capture_device: Arc::new(RwLock::new(default_capture_device)),
                capture_settings: RwLock::new(CaptureSettings {interval: 1000})
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::list_inputs, commands::set_capture_device, commands::set_capture_interval, commands::start_capture, commands::stop_capture])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
