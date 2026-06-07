use crate::core_modules::network;
use crate::models::types::NetworkInfo;

#[tauri::command]
pub async fn get_network_info() -> Result<NetworkInfo, String> {
    network::get_network_info()
        .await
        .map_err(|e| e.to_string())
}
