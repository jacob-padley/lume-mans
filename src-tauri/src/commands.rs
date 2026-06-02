use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, State};
use tokio::time::interval;

use crate::detection::source::DetectionSource;
use crate::detection::state::TrackState;
use crate::detection::video::{VideoSource, VideoSourceOption};
use crate::AppState;

#[tauri::command]
pub fn list_inputs() -> Result<Vec<VideoSourceOption>, String> {
    VideoSourceOption::all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_capture_device(id: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut capture_device = state.capture_source.write().unwrap();
    *capture_device = VideoSource::new(
        VideoSourceOption::from_id(id).map_err(|e| e.to_string())?,
        state.ocr_engine.clone(),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn set_capture_interval(interval: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = state.capture_settings.write().unwrap();
    settings.interval = interval;

    Ok(())
}

#[tauri::command]
pub async fn start_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let is_capturing = state.capture_active.clone();

    if is_capturing.load(Ordering::SeqCst) {
        return Err(String::from("Capture is already active"));
    }

    is_capturing.store(true, Ordering::SeqCst);

    let settings = state.capture_settings.read().unwrap();

    let capture_source_lock = state.capture_source.clone();
    let state_manager_lock = state.state_manager.clone();
    let interval_millis = settings.interval;

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(interval_millis as u64));
        loop {
            if !is_capturing.load(Ordering::SeqCst) {
                break;
            }
            ticker.tick().await;

            let capture_source = capture_source_lock.read().unwrap();
            let maybe_detected_state: anyhow::Result<TrackState>;
            {
                let state_manager = state_manager_lock.read().unwrap();
                let current_state = state_manager.get_state();
                maybe_detected_state = capture_source.get_track_state(current_state);
            }
            match maybe_detected_state {
                Ok(state) => {
                    let mut state_manager = state_manager_lock.write().unwrap();
                    state_manager.set_state(state, &app);
                }
                Err(_) => (),
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.capture_active.store(false, Ordering::SeqCst);

    Ok(())
}
