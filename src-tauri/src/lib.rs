mod commands;
mod detection;

use ocrs::{OcrEngine, OcrEngineParams};
use rten::Model;
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use tauri::{path::BaseDirectory, Emitter, Manager};

use crate::detection::state_machine::{TrackStateMachine, VideoStateMachine};
use crate::detection::video::{VideoSource, VideoSourceOption};

struct AppState {
    ocr_engine: Arc<OcrEngine>,
    capture_active: Arc<AtomicBool>,
    capture_source: Arc<RwLock<VideoSource>>,
    state_machine: Arc<RwLock<dyn TrackStateMachine + Send + Sync>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Workaround for crash on Wayland + Nvidia drivers
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .setup(|app| {
            let detection_model_path = app
                .path()
                .resolve("resources/text-detection.rten", BaseDirectory::Resource)
                .expect("Failed to load text detection model");
            let recognition_model_path = app
                .path()
                .resolve("resources/text-recognition.rten", BaseDirectory::Resource)
                .expect("Failed to load text recognition model");

            let detection_model = Model::load_file(detection_model_path)?;
            let recognition_model = Model::load_file(recognition_model_path)?;

            let ocr_engine = Arc::new(
                OcrEngine::new(OcrEngineParams {
                    detection_model: Some(detection_model),
                    recognition_model: Some(recognition_model),
                    ..Default::default()
                })
                .expect("Failed to initialise OCR engine"),
            );

            let default_capture_source =
                VideoSource::new(VideoSourceOption::primary(), ocr_engine.clone())
                    .expect("Failed to initialize primary video capture source");

            let state_machine = VideoStateMachine::new();
            let handle = app.handle().clone();
            let mut state_receiver = state_machine.subscribe();

            // Start a tokio task to handle changes in track state
            tauri::async_runtime::spawn(async move {
                while let Ok(new_state) = state_receiver.recv().await {
                    let _ = handle.emit("track-status", new_state);
                }
            });

            app.manage(AppState {
                ocr_engine: ocr_engine.clone(),
                capture_active: Arc::new(AtomicBool::new(false)),
                capture_source: Arc::new(RwLock::new(default_capture_source)),
                state_machine: Arc::new(RwLock::new(state_machine)),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_inputs,
            commands::set_capture_device,
            commands::start_capture,
            commands::override_status,
            commands::stop_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
