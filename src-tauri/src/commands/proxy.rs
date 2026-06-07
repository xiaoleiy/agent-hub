use crate::core_modules::proxy;
use crate::models::types::ProxyInfo;

#[tauri::command]
pub fn get_proxy_info() -> ProxyInfo {
    proxy::get_proxy_info()
}
