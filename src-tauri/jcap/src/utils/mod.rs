#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "linux")]
mod linux;

/// Checks if process has permission to capture the screen
#[cfg(target_os = "macos")]
pub fn has_permission() -> bool {
    mac::has_permission()
}
#[cfg(target_os = "windows")]
pub fn has_permission() -> bool {
    win::has_permission()
}
#[cfg(target_os = "linux")]
pub fn has_permission() -> bool {
    linux::has_permission()
}

/// Prompts user to grant screen capturing permission to current process
#[cfg(target_os = "macos")]
pub fn request_permission() -> bool {
    mac::request_permission()
}
#[cfg(target_os = "windows")]
pub fn request_permission() -> bool {
    win::request_permission()
}
#[cfg(target_os = "linux")]
pub fn request_permission() -> bool {
    linux::request_permission()
}

/// Checks if scap is supported on the current system
#[cfg(target_os = "macos")]
pub fn is_supported() -> bool {
    mac::is_supported()
}
#[cfg(target_os = "windows")]
pub fn is_supported() -> bool {
    win::is_supported()
}
#[cfg(target_os = "linux")]
pub fn is_supported() -> bool {
    linux::is_supported()
}
