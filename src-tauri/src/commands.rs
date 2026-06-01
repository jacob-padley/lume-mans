use crate::detection::video::VideoInput;

#[tauri::command]
pub fn list_inputs() -> Result<Vec<VideoInput>, String> {
    VideoInput::all().map_err(|e| e.to_string())
}