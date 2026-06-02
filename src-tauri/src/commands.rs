use ocrs::ImageSource;
use tauri::State;
use tokio::time::interval;
use xcap::image::EncodableLayout;
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
    let mut capture_device = state.capture_device.write().unwrap();
    *capture_device = VideoInput::from_id(id).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn set_capture_interval(interval: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = state.capture_settings.write().unwrap();
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

    let settings = state.capture_settings.read().unwrap();

    let ocr_engine = state.ocr_engine.clone();
    let capture_monitor_lock = state.capture_device.clone();
    let capture_monitor = capture_monitor_lock.read().unwrap().get_monitor().expect("Could not acquire display device");
    let interval_millis = settings.interval;

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(interval_millis as u64));
        println!("Capture started");
        loop {
            if !is_capturing.load(Ordering::SeqCst) {
                break;
            }
            ticker.tick().await;

            // TODO: fail gracefully if any unwraps fail
            println!("Taking screenshot");
            let image = capture_monitor.capture_image().expect("Image capture failed");
            println!("Running OCR");
            let image_source = ImageSource::from_bytes(image.as_bytes(), image.dimensions()).unwrap();
            let ocr_input = ocr_engine.prepare_input(image_source).unwrap();

            println!("{}", ocr_engine.get_text(&ocr_input).unwrap());
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.capture_active.store(false, Ordering::SeqCst);

    Ok(())
}
