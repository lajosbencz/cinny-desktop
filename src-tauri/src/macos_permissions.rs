#[cfg(target_os = "macos")]
extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

#[tauri::command]
pub fn check_input_monitoring() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { CGPreflightListenEventAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
pub fn request_input_monitoring() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { CGRequestListenEventAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}
