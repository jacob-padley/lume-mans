use tauri::State;
use tokio::time::interval;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::detection::video::VideoInput;
use crate::AppState;

#[tauri::command]
pub fn list_inputs() -> Result<Vec<VideoInput>, String> {
    VideoInput::all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_capture_device(id: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = state.capture_settings.lock().unwrap();
    settings.device = id;

    Ok(())
}

#[tauri::command]
pub fn set_capture_interval(interval: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = state.capture_settings.lock().unwrap();
    settings.interval = interval;

    Ok(())
}

#[tauri::command]
pub async fn start_capture(state: State<'_, AppState>) -> Result<(), String> {
    let is_capturing = state.capture_active.clone();

    if is_capturing.load(Ordering::SeqCst) {
        return Err(String::from("Capture is already active"))
    }

    is_capturing.store(true, Ordering::SeqCst);

    let settings = state.capture_settings.lock().unwrap();

    let ocr_engine = state.ocr_engine.clone();
    let interval_millis = settings.interval;

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(interval_millis as u64));

        println!("Capture started");

        loop {
            if !is_capturing.load(Ordering::SeqCst) {
                println!("Capture stopped");
                break;
            }

            ticker.tick().await;

            println!("tick");
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.capture_active.store(false, Ordering::SeqCst);

    Ok(())
}
