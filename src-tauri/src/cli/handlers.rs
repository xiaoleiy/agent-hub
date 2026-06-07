use crate::core_modules::{agents, keepalive, network, system};
use crate::models::types::AgentType;
use colored::*;

fn parse_agent(name: &str) -> Option<AgentType> {
    match name.to_lowercase().as_str() {
        "claude" | "claude code" => Some(AgentType::ClaudeCode),
        "cursor" => Some(AgentType::Cursor),
        "codex" => Some(AgentType::Codex),
        _ => None,
    }
}

pub fn status(json: bool) {
    // First call seeds the snapshot; second call computes the rate
    let _ = system::get_system_status();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let status = system::get_system_status();

    if json {
        println!("{}", serde_json::to_string_pretty(&status).unwrap());
        return;
    }

    println!("{}", "System Status".bold().underline());
    println!(
        "  {} {}",
        "CPU:".dimmed(),
        format!("{:.1}% ({} cores)", status.cpu_usage, status.cpu_cores).cyan()
    );
    println!(
        "  {} {}",
        "RAM:".dimmed(),
        format!(
            "{:.1} / {:.1} GB ({:.1}%)",
            status.ram_used_gb, status.ram_total_gb, status.ram_usage_percent
        )
        .cyan()
    );
    println!(
        "  {} {}",
        "Uptime:".dimmed(),
        status.uptime_formatted.cyan()
    );
    println!(
        "  {} {}",
        "User:".dimmed(),
        status.username.cyan()
    );
    println!(
        "  {} {}",
        "Upload:".dimmed(),
        format_rate(status.network_upload_rate).cyan()
    );
    println!(
        "  {} {}",
        "Download:".dimmed(),
        format_rate(status.network_download_rate).cyan()
    );
}

pub fn network(json: bool) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(network::get_network_info());

    match result {
        Ok(info) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&info).unwrap());
                return;
            }

            println!("{}", "Network Information".bold().underline());
            println!("  {} {}", "IP:".dimmed(), info.ip.green().bold());
            if !info.country.is_empty() {
                println!(
                    "  {} {}",
                    "Location:".dimmed(),
                    format!("{}, {}", info.city, info.region)
                );
            }
            if !info.org.is_empty() {
                println!("  {} {}", "ISP:".dimmed(), info.org);
            }
            if !info.timezone.is_empty() {
                println!("  {} {}", "Timezone:".dimmed(), info.timezone);
            }
        }
        Err(e) => {
            eprintln!("{} Failed to fetch network info: {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

pub fn agents(json: bool) {
    let agents = agents::detect_all_agents();

    if json {
        println!("{}", serde_json::to_string_pretty(&agents).unwrap());
        return;
    }

    println!("{}", "Detected Agents".bold().underline());
    for agent in &agents {
        let status_icon = if agent.running {
            "●".green()
        } else if agent.installed {
            "○".yellow()
        } else {
            "✕".dimmed()
        };

        let version_str = agent
            .version
            .as_ref()
            .map(|v| format!("v{}", v))
            .unwrap_or_else(|| "unknown".to_string());

        println!(
            "  {} {} {} — {}",
            status_icon,
            agent.name.bold(),
            version_str.dimmed(),
            if agent.running {
                format!("{} active session(s)", agent.active_sessions).green()
            } else if agent.installed {
                "installed, not running".yellow()
            } else {
                "not installed".dimmed()
            }
        );
    }
}

pub fn sessions(agent_filter: Option<String>, json: bool) {
    let all_agents = agents::detect_all_agents();
    let mut all_sessions = Vec::new();

    for agent in &all_agents {
        if let Some(ref filter) = agent_filter {
            if let Some(at) = parse_agent(filter) {
                if agent.agent_type != at {
                    continue;
                }
            }
        }
        let mut sessions = agents::get_sessions(&agent.agent_type);
        all_sessions.append(&mut sessions);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&all_sessions).unwrap());
        return;
    }

    println!("{}", "Active Sessions".bold().underline());
    if all_sessions.is_empty() {
        println!("  {}", "No active sessions".dimmed());
        return;
    }

    for sess in &all_sessions {
        let status_str = match sess.status.as_str() {
            "busy" => sess.status.red(),
            "idle" | "completed" => sess.status.green(),
            _ => sess.status.normal(),
        };

        let cwd = sess
            .working_dir
            .as_ref()
            .map(|d| format!(" {}", d.dimmed()))
            .unwrap_or_default();

        println!(
            "  {} {} — {}{}",
            sess.agent.bold(),
            sess.id.chars().take(12).collect::<String>().dimmed(),
            status_str,
            cwd
        );
    }
}

pub fn usage(window: &str, agent_filter: Option<String>, json: bool) {
    let all_agents = agents::detect_all_agents();
    let mut all_usage = Vec::new();

    for agent in &all_agents {
        if let Some(ref filter) = agent_filter {
            if let Some(at) = parse_agent(filter) {
                if agent.agent_type != at {
                    continue;
                }
            }
        }
        let usage = agents::get_usage(&agent.agent_type, window);
        all_usage.push(usage);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&all_usage).unwrap());
        return;
    }

    println!(
        "{}",
        format!("Usage Statistics (window: {})", window).bold().underline()
    );

    for u in &all_usage {
        println!(
            "  {} {} sessions, {} interactions",
            u.agent.bold(),
            u.total_sessions.to_string().cyan(),
            u.total_interactions.to_string().cyan()
        );
    }
}

pub fn keepalive(mode: Option<String>, status_flag: bool) {
    if status_flag || mode.is_none() {
        let status = keepalive::get_keepalive_status();
        println!("{}", "Keep-Alive Status".bold().underline());
        if status.active {
            println!("  {} {}", "Status:".dimmed(), "ACTIVE".green().bold());
            println!(
                "  {} {}",
                "Mode:".dimmed(),
                status.mode.unwrap_or_default().cyan()
            );
            if let Some(ref started) = status.started_at {
                println!("  {} {}", "Started:".dimmed(), started.dimmed());
            }
            if let Some(ref expires) = status.expires_at {
                println!("  {} {}", "Expires:".dimmed(), expires.dimmed());
            }
        } else {
            println!("  {} {}", "Status:".dimmed(), "OFF".dimmed());
        }
        return;
    }

    let mode = mode.unwrap();

    if mode == "off" || mode == "0" {
        match keepalive::stop_keepalive() {
            Ok(()) => println!("{}", "Keep-alive disabled".green()),
            Err(e) => {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }
    } else {
        match keepalive::start_keepalive(&mode) {
            Ok(()) => println!(
                "{} Keep-alive started (mode: {})",
                "✓".green().bold(),
                mode.cyan()
            ),
            Err(e) => {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
        }
    }
}

#[allow(dead_code)]
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_048_576.0 {
        format!("{:.2} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.2} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}
