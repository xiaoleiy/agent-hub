use serde::{Deserialize, Serialize};

/// System status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub ram_usage_percent: f64,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub username: String,
    pub hostname: String,
    pub network_upload_bytes: u64,
    pub network_download_bytes: u64,
    pub network_upload_rate: f64,
    pub network_download_rate: f64,
}

/// Network information from external IP lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub ip: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub timezone: String,
}

/// An AI agent detected on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub agent_type: AgentType,
    pub installed: bool,
    pub running: bool,
    pub active_sessions: usize,
    pub cli_sessions: usize,
    pub gui_sessions: usize,
    /// Primary version (for backward compat)
    pub version: Option<String>,
    pub cli_version: Option<String>,
    pub gui_version: Option<String>,
    pub install_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    ClaudeCode,
    Cursor,
    Codex,
}

/// A single agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub agent: String,
    pub status: String,
    pub started_at: Option<String>,
    pub working_dir: Option<String>,
    pub model: Option<String>,
    pub pid: Option<u32>,
    /// "cli", "gui", "vscode", "sdk", etc.
    pub entrypoint: String,
}

/// Usage statistics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub agent: String,
    pub window: String,
    pub total_sessions: usize,
    pub total_interactions: usize,
    pub first_activity: Option<String>,
    pub last_activity: Option<String>,
}

/// Keep-alive status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepAliveStatus {
    pub active: bool,
    pub mode: Option<String>,
    pub started_at: Option<String>,
    pub expires_at: Option<String>,
    pub pid: Option<u32>,
}
