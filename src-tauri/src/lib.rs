mod commands;
mod detection;
mod lighting;

use ocrs::{OcrEngine, OcrEngineParams};
use rten::Model;
use std::net::Ipv4Addr;
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use tauri::{path::BaseDirectory, Emitter, Manager};

use crate::detection::video::{VideoSource, VideoSourceOption, VideoStateMachine};
use crate::detection::{TrackState, TrackStateMachine};
use crate::lighting::command::{
    LightingCommand, PlayPlaybackCommand, PlaybackHandle, ReleaseAllPlaybacksCommand,
};
use crate::lighting::controller::LightingController;

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
            let emit_app_handle = app.handle().clone();
            let mut emit_state_receiver = state_machine.subscribe();

            // Start a tokio task to handle changes in track state
            tauri::async_runtime::spawn(async move {
                while let Ok(new_state) = emit_state_receiver.recv().await {
                    let _ = emit_app_handle.emit("track-status", new_state);
                }
            });

            let mut lighting_state_receiver = state_machine.subscribe();
            let lighting_app_handle = app.handle().clone();
            // Use this channel to thread the track status callback from the HTTP requests to the
            // lighting API in case they block for a long time
            tauri::async_runtime::spawn(async move {
                let lighting_controller = LightingController::new(Ipv4Addr::LOCALHOST, 4430);
                while let Ok(new_state) = lighting_state_receiver.recv().await {
                    let playback_handle = match new_state {
                        TrackState::GreenFlag => PlaybackHandle::UserNumber(4),
                        TrackState::SafetyCar | TrackState::VirtualSafetyCar => {
                            PlaybackHandle::UserNumber(1)
                        }
                        TrackState::YellowFlag | TrackState::SlowZone => {
                            PlaybackHandle::UserNumber(2)
                        }
                        TrackState::RedFlag => PlaybackHandle::UserNumber(3),
                        TrackState::SafetyCarEnding
                        | TrackState::FullCourseYellowEnding
                        | TrackState::VirtualSafetyCarEnding => PlaybackHandle::UserNumber(6),
                        TrackState::FullCourseYellow => PlaybackHandle::UserNumber(7),
                        TrackState::CheckeredFlag => PlaybackHandle::UserNumber(8),
                        _ => continue,
                    };

                    // Release all playbacks first
                    if let Err(e) = lighting_controller
                        .send(LightingCommand::ReleaseAllPlaybacks(
                            ReleaseAllPlaybacksCommand {
                                ..Default::default()
                            },
                        ))
                        .await
                    {
                        eprintln!("ReleaseAllPlaybacks failed: {}", e);
                        let _ = lighting_app_handle.emit("error", e.to_string());
                        continue;
                    }
                    // Now play the lighting command
                    if let Err(e) = lighting_controller
                        .send(LightingCommand::PlayPlayback(
                            PlayPlaybackCommand::from_handle(playback_handle),
                        ))
                        .await
                    {
                        eprintln!("PlayPlayback failed: {}", e);
                        let _ = lighting_app_handle.emit("error", e.to_string());
                    }
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
