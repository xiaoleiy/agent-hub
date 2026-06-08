use crate::core_modules::agents;
use crate::models::types::{AgentInfo, AgentType, AgentUsage, Session, UsageStats};

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
        "OpenCode" | "opencode" => AgentType::OpenCode,
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
        "OpenCode" | "opencode" => AgentType::OpenCode,
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

#[tauri::command]
pub fn get_agent_rich_usage(agent: String) -> AgentUsage {
    let agent_type = match agent.as_str() {
        "Claude Code" | "claude" => AgentType::ClaudeCode,
        "Cursor" | "cursor" => AgentType::Cursor,
        "Codex" | "codex" => AgentType::Codex,
        "OpenCode" | "opencode" => AgentType::OpenCode,
        _ => {
            return AgentUsage {
                agent,
                session_window: None,
                weekly_window: None,
                tokens: None,
                model_breakdowns: vec![],
                total_interactions: 0,
                total_sessions: 0,
            }
        }
    };
    agents::get_rich_usage(&agent_type)
}
