use crate::models::types::NetworkInfo;
use anyhow::Result;
use std::time::Duration;

/// Fetch public network information from ipinfo.io.
/// Bounded by a timeout so a hung request can't stall the caller indefinitely.
pub async fn get_network_info() -> Result<NetworkInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let resp = client.get("https://ipinfo.io/json").send().await?;
    let info: NetworkInfo = resp.json().await?;
    Ok(info)
}
