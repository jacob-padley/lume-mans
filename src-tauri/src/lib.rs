mod commands;
mod detection;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Workaround for crash on Wayland + Nvidia drivers
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::list_inputs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
