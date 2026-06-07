use crate::models::types::NetworkInfo;
use anyhow::Result;

/// Fetch public network information from ip.net.coffee
pub async fn get_network_info() -> Result<NetworkInfo> {
    let resp = reqwest::get("https://ip.net.coffee").await?;
    let info: NetworkInfo = resp.json().await?;
    Ok(info)
}
