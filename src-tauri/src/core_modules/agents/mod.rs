pub mod claude;
pub mod codex;
pub mod cursor;

use crate::models::types::{AgentInfo, AgentType, AgentUsage, Session, UsageStats};

/// Detect all agents and return their info
pub fn detect_all_agents() -> Vec<AgentInfo> {
    vec![
        claude::detect(),
        cursor::detect(),
        codex::detect(),
    ]
}

/// Get sessions for a specific agent type
pub fn get_sessions(agent_type: &AgentType) -> Vec<Session> {
    match agent_type {
        AgentType::ClaudeCode => claude::get_sessions(),
        AgentType::Cursor => cursor::get_sessions(),
        AgentType::Codex => codex::get_sessions(),
    }
}

/// Get usage stats for a specific agent and time window
pub fn get_usage(agent_type: &AgentType, window: &str) -> UsageStats {
    match agent_type {
        AgentType::ClaudeCode => claude::get_usage(window),
        AgentType::Cursor => cursor::get_usage(window),
        AgentType::Codex => codex::get_usage(window),
    }
}

/// Get rich usage data with token breakdowns and rate limits
pub fn get_rich_usage(agent_type: &AgentType) -> AgentUsage {
    match agent_type {
        AgentType::ClaudeCode => claude::get_rich_usage(),
        AgentType::Cursor => cursor::get_rich_usage(),
        AgentType::Codex => codex::get_rich_usage(),
    }
}
