use crate::models::types::NetworkInfo;
use anyhow::Result;

/// Fetch public network information from ipinfo.io
pub async fn get_network_info() -> Result<NetworkInfo> {
    let resp = reqwest::get("https://ipinfo.io/json").await?;
    let info: NetworkInfo = resp.json().await?;
    Ok(info)
}
