use crate::core_modules::system;
use crate::models::types::SystemStatus;

#[tauri::command]
pub fn get_system_status() -> SystemStatus {
    system::get_system_status()
}
