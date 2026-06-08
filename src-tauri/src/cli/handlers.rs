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
        "Host:".dimmed(),
        status.hostname.cyan()
    );
    println!(
        "  {}",
        "Traffic:".dimmed()
    );
    println!(
        "    {} {}",
        "↑ Upload:".dimmed(),
        format_rate(status.network_upload_rate).cyan()
    );
    println!(
        "    {} {}",
        "↓ Download:".dimmed(),
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

        // Build version display
        let version_str = match (&agent.cli_version, &agent.gui_version) {
            (Some(cv), Some(gv)) if cv != gv => {
                format!("CLI v{} / GUI v{}", cv, gv)
            }
            (Some(cv), Some(_gv)) => format!("v{}", cv),
            (Some(cv), None) => format!("CLI v{}", cv),
            (None, Some(gv)) => format!("GUI v{}", gv),
            _ => "unknown".to_string(),
        };

        let session_detail = if agent.running {
            let mut parts = Vec::new();
            if agent.cli_sessions > 0 {
                parts.push(format!("{} CLI", agent.cli_sessions));
            }
            if agent.gui_sessions > 0 {
                parts.push(format!("{} GUI", agent.gui_sessions));
            }
            format!("{} active ({})", agent.active_sessions, parts.join(", ")).green()
        } else if agent.installed {
            "Not Opened".yellow()
        } else {
            "Not Found".dimmed()
        };

        println!(
            "  {} {} {} — {}",
            status_icon,
            agent.name.bold(),
            version_str.dimmed(),
            session_detail
        );

        if let Some(acc) = &agent.account {
            if let Some(who) = acc.email.as_deref().or(acc.display_name.as_deref()) {
                let org = acc
                    .organization
                    .as_deref()
                    .map(|o| format!(" ({})", o))
                    .unwrap_or_default();
                println!("      {} {}{}", "Account:".dimmed(), who.green(), org.dimmed());
            }
        }
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

        let ep = if sess.entrypoint.is_empty() {
            "—".dimmed()
        } else {
            match sess.entrypoint.as_str() {
                "cli" | "sdk-cli" => sess.entrypoint.cyan(),
                "vscode" | "exec" => sess.entrypoint.magenta(),
                _ => sess.entrypoint.dimmed(),
            }
        };

        let cwd = sess
            .working_dir
            .as_ref()
            .map(|d| format!(" {}", d.dimmed()))
            .unwrap_or_default();

        println!(
            "  {} {} [{}] — {}{}",
            sess.agent.bold(),
            sess.id.chars().take(12).collect::<String>().dimmed(),
            ep,
            status_str,
            cwd
        );
    }
}

pub fn usage(window: &str, agent_filter: Option<String>, json: bool) {
    let all_agents = agents::detect_all_agents();
    let mut all_usage = Vec::new();
    let mut all_rich = Vec::new();

    for agent in &all_agents {
        if let Some(ref filter) = agent_filter {
            if let Some(at) = parse_agent(filter) {
                if agent.agent_type != at {
                    continue;
                }
            }
        }
        all_usage.push(agents::get_usage(&agent.agent_type, window));
        all_rich.push(agents::get_rich_usage(&agent.agent_type));
    }

    if json {
        // Rich usage carries rate-limit windows + token breakdowns.
        println!("{}", serde_json::to_string_pretty(&all_rich).unwrap());
        return;
    }

    println!(
        "{}",
        format!("Usage Statistics (window: {})", window).bold().underline()
    );

    for (u, r) in all_usage.iter().zip(all_rich.iter()) {
        println!(
            "  {} {} sessions, {} interactions",
            u.agent.bold(),
            u.total_sessions.to_string().cyan(),
            u.total_interactions.to_string().cyan()
        );
        if let Some(w) = &r.session_window {
            println!("      {} {}", "Session limit:".dimmed(), format_rate_window(w));
        }
        if let Some(w) = &r.weekly_window {
            println!("      {} {}", "Weekly limit: ".dimmed(), format_rate_window(w));
        }
        if let Some(t) = &r.tokens {
            println!(
                "      {} {} total ({} in / {} out)",
                "Tokens:".dimmed(),
                t.total_tokens.to_string().cyan(),
                t.input_tokens,
                t.output_tokens
            );
        }
    }
}

fn format_rate_window(w: &crate::models::types::RateWindow) -> String {
    let label = if w.window_minutes >= 43200 {
        format!("{}mo", w.window_minutes / 43200)
    } else if w.window_minutes >= 10080 {
        format!("{}w", w.window_minutes / 10080)
    } else if w.window_minutes >= 60 {
        format!("{}h", w.window_minutes / 60)
    } else {
        format!("{}m", w.window_minutes)
    };
    let reset = w
        .resets_at
        .as_deref()
        .map(|r| format!(", resets {}", r))
        .unwrap_or_default();
    format!("{:.0}% used ({} window{})", w.used_percent, label, reset)
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
