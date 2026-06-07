use crate::core_modules::keepalive;
use crate::models::types::KeepAliveStatus;

#[tauri::command]
pub fn get_keepalive_status() -> KeepAliveStatus {
    keepalive::get_keepalive_status()
}

#[tauri::command]
pub fn set_keepalive(mode: String) -> Result<(), String> {
    if mode == "off" || mode == "0" {
        keepalive::stop_keepalive()
    } else {
        keepalive::start_keepalive(&mode)
    }
}
