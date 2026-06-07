use crate::core_modules::agents;
use crate::models::types::{AgentInfo, AgentType, Session, UsageStats};

#[tauri::command]
pub fn get_agents() -> Vec<AgentInfo> {
    agents::detect_all_agents()
}

#[tauri::command]
pub fn get_agent_sessions(agent: String) -> Vec<Session> {
    let agent_type = match agent.as_str() {
        "Claude Code" | "claude" => AgentType::ClaudeCode,
        "Cursor" | "cursor" => AgentType::Cursor,
        "Codex" | "codex" => AgentType::Codex,
        _ => return vec![],
    };
    agents::get_sessions(&agent_type)
}

#[tauri::command]
pub fn get_agent_usage(agent: String, window: Option<String>) -> UsageStats {
    let agent_type = match agent.as_str() {
        "Claude Code" | "claude" => AgentType::ClaudeCode,
        "Cursor" | "cursor" => AgentType::Cursor,
        "Codex" | "codex" => AgentType::Codex,
        _ => {
            return UsageStats {
                agent,
                window: "5h".to_string(),
                total_sessions: 0,
                total_interactions: 0,
                first_activity: None,
                last_activity: None,
            }
        }
    };
    agents::get_usage(&agent_type, &window.unwrap_or_else(|| "5h".to_string()))
}
