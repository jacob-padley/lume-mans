use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tokio::time::{interval, MissedTickBehavior};

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
pub async fn start_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let is_capturing = state.capture_active.clone();

    if is_capturing.load(Ordering::SeqCst) {
        return Err(String::from("Capture is already active"));
    }

    is_capturing.store(true, Ordering::SeqCst);

    let capture_source_lock = state.capture_source.clone();
    let state_manager_lock = state.state_manager.clone();

    let mut last_tick = Instant::now();

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(200));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            if !is_capturing.load(Ordering::SeqCst) {
                return;
            }
            ticker.tick().await;

            // Emit the last frame time to display on the frontend
            let now = Instant::now();
            let elapsed = now.duration_since(last_tick);
            last_tick = now;
            let frame_millis = elapsed.as_millis();
            app.emit("last-frame-time", frame_millis).unwrap();

            // Capture and process the next frame
            let capture_source = capture_source_lock.read().unwrap();
            let maybe_detected_state: anyhow::Result<TrackState>;
            {
                let state_manager = state_manager_lock.read().unwrap();
                let current_state = state_manager.get_state();
                maybe_detected_state = capture_source.get_track_state(current_state);
            }
            if let Ok(state) = maybe_detected_state {
                let mut state_manager = state_manager_lock.write().unwrap();
                state_manager.set_state(state, &app);
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
