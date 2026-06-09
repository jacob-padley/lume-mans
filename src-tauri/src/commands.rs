use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tokio::time::{interval, MissedTickBehavior};

use crate::detection::source::DetectionSource;
use crate::detection::state::{SessionTime, TrackState};
use crate::detection::video::{VideoSource, VideoSourceOption, VideoSourceType};
use crate::AppState;

#[tauri::command]
pub fn list_inputs() -> Result<Vec<VideoSourceOption>, String> {
    Ok(VideoSourceOption::all())
}

#[tauri::command]
pub async fn set_capture_device(
    id: u32,
    source_type: VideoSourceType,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut capture_device = state.capture_source.write().unwrap();
    *capture_device = VideoSource::new(
        VideoSourceOption::get(id, source_type).map_err(|e| e.to_string())?,
        state.ocr_engine.clone(),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn start_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let is_capturing = state.capture_active.clone();

    if is_capturing.load(Ordering::SeqCst) {
        return Err(String::from("Capture is already active"));
    }

    is_capturing.store(true, Ordering::SeqCst);

    let capture_source_lock = state.capture_source.clone();
    let state_machine_lock = state.state_machine.clone();

    let mut last_tick = Instant::now();

    {
        let capture_source = capture_source_lock.write().unwrap();
        capture_source.start_capture().map_err(|e| e.to_string())?;
    }
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_millis(100));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            if !is_capturing.load(Ordering::SeqCst) {
                break;
            }
            ticker.tick().await;

            // Emit the last frame time to display on the frontend
            let now = Instant::now();
            let elapsed = now.duration_since(last_tick);
            last_tick = now;
            let frame_millis = elapsed.as_millis();
            let _ = app.emit("last-frame-time", frame_millis);

            // Capture and process the next frame
            let maybe_state: Option<TrackState>;
            let maybe_timer: Option<SessionTime>;
            // Enclose in a scope to close mutexes as early as possible
            {
                let mut capture_source = capture_source_lock.write().unwrap();
                // Get whatever state the video source is able to provide for this frame
                match capture_source.get_track_state() {
                    Some((state, timer)) => {
                        maybe_state = state;
                        maybe_timer = timer;
                    }
                    _ => continue,
                }
            }
            // Broadcast the race time to the frontend if available
            if let Some(time) = maybe_timer {
                let _ = app.emit("session-time", time);
            }
            // Allow the state manager to decide if the reported state necessitates a state
            // transition.
            let mut state_machine = state_machine_lock.write().unwrap();
            state_machine.handle_state(maybe_state, maybe_timer);
        }
        let capture_source = capture_source_lock.write().unwrap();
        capture_source.stop_capture();
    });
    Ok(())
}

#[tauri::command]
pub async fn override_status(status: String, state: State<'_, AppState>) -> Result<(), String> {
    let target_state: TrackState;
    if status == "GreenFlag" {
        target_state = TrackState::GreenFlag;
    } else if status == "SessionStart" {
        target_state = TrackState::SessionStart;
    } else {
        return Err(String::from("Invalid state"));
    }

    let state_machine_lock = state.state_machine.clone();
    let mut state_machine = state_machine_lock.write().unwrap();
    state_machine.override_state(target_state);

    Ok(())
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.capture_active.store(false, Ordering::SeqCst);

    Ok(())
}
