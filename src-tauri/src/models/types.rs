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

/// Usage statistics for an agent (legacy simple format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub agent: String,
    pub window: String,
    pub total_sessions: usize,
    pub total_interactions: usize,
    pub first_activity: Option<String>,
    pub last_activity: Option<String>,
}

/// Rich usage data matching codexbar's data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    pub agent: String,
    /// 5-hour session rate limit window
    pub session_window: Option<RateWindow>,
    /// Weekly rate limit window
    pub weekly_window: Option<RateWindow>,
    /// Token breakdown from local JSONL parsing
    pub tokens: Option<TokenUsage>,
    /// Per-model token breakdown
    pub model_breakdowns: Vec<ModelUsage>,
    /// Total interactions counted from history
    pub total_interactions: usize,
    /// Number of sessions
    pub total_sessions: usize,
}

/// A rate-limit window (like codexbar's RateWindow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateWindow {
    /// Percent used (0-100)
    pub used_percent: f64,
    /// Window duration in minutes (e.g. 300 for 5h, 10080 for weekly)
    pub window_minutes: u64,
    /// When the window resets (ISO 8601)
    pub resets_at: Option<String>,
}

/// Token usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_create_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Per-model usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub request_count: usize,
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

/// Full proxy/VPN information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub system_proxy: SystemProxy,
    pub vpn_connections: Vec<VpnConnection>,
    pub active_client: Option<ProxyClient>,
    pub proxy_nodes: Vec<ProxyNode>,
}

/// System-level proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProxy {
    pub active_service: String,
    pub http: ProxyEntry,
    pub https: ProxyEntry,
    pub socks: ProxyEntry,
    pub pac: Option<String>,
    pub bypass: Vec<String>,
}

/// A single proxy entry (HTTP, HTTPS, or SOCKS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyEntry {
    pub enabled: bool,
    pub server: String,
    pub port: u16,
}

/// A VPN connection detected by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConnection {
    pub name: String,
    pub connected: bool,
    pub vpn_type: String,
}

/// A detected proxy client (Clash, Surge, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyClient {
    pub name: String,
    pub client_type: String,
    pub api_port: u16,
    pub version: Option<String>,
    pub mode: Option<String>,
}

/// A proxy node/group from the active client's API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    pub selected: String,
    pub delay: Option<u64>,
    pub node_type: String,
    pub available_nodes: Vec<String>,
}
