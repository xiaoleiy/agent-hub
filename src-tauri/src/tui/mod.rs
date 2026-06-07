use crate::core_modules::{agents, keepalive, network, proxy, system};
use crate::models::types::{AgentInfo, AgentUsage, KeepAliveStatus, NetworkInfo, ProxyInfo, Session, SystemStatus, UsageStats};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Cell, Gauge, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

const MIN_DASHBOARD_WIDTH_FOR_SIDE_BY_SIDE_AGENTS: u16 = 140;

struct DashboardLayout {
    system: Rect,
    network: Rect,
    agents: Rect,
    usage: Rect,
}

struct App {
    system: SystemStatus,
    network: Option<NetworkInfo>,
    agents: Vec<AgentInfo>,
    /// Sessions grouped by agent index
    agent_sessions: Vec<Vec<Session>>,
    /// Usage stats per agent
    agent_usage_5h: Vec<UsageStats>,
    agent_usage_1w: Vec<UsageStats>,
    agent_rich_usage: Vec<AgentUsage>,
    proxy: ProxyInfo,
    keepalive: KeepAliveStatus,
    active_tab: usize,
    last_refresh: Instant,
    network_loaded: bool,
}

impl App {
    fn new() -> Self {
        let agents = agents::detect_all_agents();
        let agent_count = agents.len();
        Self {
            system: system::get_system_status(),
            network: None,
            agents,
            agent_sessions: vec![Vec::new(); agent_count],
            agent_usage_5h: Vec::new(),
            agent_usage_1w: Vec::new(),
            agent_rich_usage: Vec::new(),
            proxy: proxy::get_proxy_info(),
            keepalive: keepalive::get_keepalive_status(),
            active_tab: 0,
            last_refresh: Instant::now(),
            network_loaded: false,
        }
    }

    /// Build tab names: ["Dashboard", <agent1>, <agent2>, ..., "Proxy / VPN", "Keep-Alive"]
    fn tab_names(&self) -> Vec<String> {
        let mut names = vec!["Dashboard".to_string()];
        for a in &self.agents {
            if a.installed {
                names.push(a.name.clone());
            }
        }
        names.push("Proxy / VPN".to_string());
        names.push("Keep-Alive".to_string());
        names
    }

    /// Installed agents only (for tab indexing)
    fn installed_agents(&self) -> Vec<&AgentInfo> {
        self.agents.iter().filter(|a| a.installed).collect()
    }

    /// Number of dynamic tabs
    fn tab_count(&self) -> usize {
        // Dashboard + installed agents + Keep-Alive
        1 + self.installed_agents().len() + 1
    }

    fn refresh(&mut self) {
        self.system = system::get_system_status();
        self.agents = agents::detect_all_agents();
        self.proxy = proxy::get_proxy_info();
        self.keepalive = keepalive::get_keepalive_status();

        // Load network once
        if !self.network_loaded {
            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Ok(info) = rt.block_on(network::get_network_info()) {
                self.network = Some(info);
                self.network_loaded = true;
            }
        }

        // Per-agent sessions
        self.agent_sessions = self
            .agents
            .iter()
            .map(|a| agents::get_sessions(&a.agent_type))
            .collect();

        // Per-agent usage (5h and 1w)
        self.agent_usage_5h = self
            .agents
            .iter()
            .map(|a| agents::get_usage(&a.agent_type, "5h"))
            .collect();
        self.agent_usage_1w = self
            .agents
            .iter()
            .map(|a| agents::get_usage(&a.agent_type, "1w"))
            .collect();

        // Rich usage with token breakdowns and rate limits
        self.agent_rich_usage = self
            .agents
            .iter()
            .map(|a| agents::get_rich_usage(&a.agent_type))
            .collect();

        // Clamp active_tab to valid range after agents list may have changed
        let count = self.tab_count();
        if count > 0 && self.active_tab >= count {
            self.active_tab = count - 1;
        }

        self.last_refresh = Instant::now();
    }

    fn next_tab(&mut self) {
        let count = self.tab_count();
        if count > 0 {
            self.active_tab = (self.active_tab + 1) % count;
        }
    }

    fn prev_tab(&mut self) {
        let count = self.tab_count();
        if count > 0 {
            self.active_tab = if self.active_tab == 0 {
                count - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    fn toggle_keepalive(&mut self) {
        if self.keepalive.active {
            let _ = keepalive::stop_keepalive();
        } else {
            let _ = keepalive::start_keepalive("1h");
        }
    }

    fn set_keepalive_mode(&mut self, mode: &str) {
        if self.keepalive.active && self.keepalive.mode.as_deref() == Some(mode) {
            let _ = keepalive::stop_keepalive();
        } else {
            let _ = keepalive::start_keepalive(mode);
        }
    }
}

pub fn run_tui() {
    // Setup terminal
    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}. Are you running in a terminal?", e);
        std::process::exit(1);
    }
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        eprintln!("Failed to setup terminal: {}", e);
        std::process::exit(1);
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    app.refresh();

    let tick_rate = Duration::from_secs(3);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &mut app)).unwrap();

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.kind == KeyEventKind::Press {
                    let tab_count = app.tab_count();
                    let last_tab = if tab_count > 0 { tab_count - 1 } else { 0 };
                    let installed = app.installed_agents();

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Right | KeyCode::Tab => app.next_tab(),
                        KeyCode::Left | KeyCode::BackTab => app.prev_tab(),
                        // '1' = Dashboard, '2'..=agents, last = Keep-Alive
                        KeyCode::Char(c @ '1'..='9') => {
                            let idx = (c as usize) - ('1' as usize);
                            if idx < tab_count {
                                app.active_tab = idx;
                            }
                        }
                        // '0' = Keep-Alive (last tab)
                        KeyCode::Char('0') => {
                            app.active_tab = last_tab;
                        }
                        // Agent shortcuts: q,w,e for first 3 agents
                        KeyCode::Char('w') => {
                            if installed.len() >= 1 {
                                app.active_tab = 1;
                            }
                        }
                        KeyCode::Char('e') => {
                            if installed.len() >= 2 {
                                app.active_tab = 2;
                            }
                        }
                        KeyCode::Char('r') => {
                            if installed.len() >= 3 {
                                app.active_tab = 3;
                            }
                        }
                        KeyCode::Char(' ') => {
                            app.toggle_keepalive();
                            app.refresh();
                        }
                        KeyCode::Char('a') => {
                            app.set_keepalive_mode("30m");
                            app.refresh();
                        }
                        KeyCode::Char('s') => {
                            app.set_keepalive_mode("1h");
                            app.refresh();
                        }
                        KeyCode::Char('d') => {
                            app.set_keepalive_mode("3h");
                            app.refresh();
                        }
                        KeyCode::Char('f') => {
                            app.set_keepalive_mode("forever");
                            app.refresh();
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.refresh();
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}

fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),   // content
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    // Tabs
    let tab_names = app.tab_names();
    let tab_count = tab_names.len();
    let last_tab = if tab_count > 0 { tab_count - 1 } else { 0 };

    // Safety: clamp active_tab to valid range (must happen before borrowing app further)
    if tab_count > 0 && app.active_tab >= tab_count {
        app.active_tab = tab_count - 1;
    }

    let installed = app.installed_agents();

    let titles: Vec<Line> = tab_names
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == app.active_tab {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(format!(" {} ", t), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Agent Hub "))
        .select(app.active_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan))
        .divider(Span::raw("│"));
    f.render_widget(tabs, chunks[0]);

    // Content
    if app.active_tab == 0 {
        draw_dashboard(f, app, chunks[1]);
    } else if app.active_tab == last_tab {
        draw_keepalive(f, app, chunks[1]);
    } else if app.active_tab == last_tab - 1 {
        draw_proxy_tab(f, &app.proxy, chunks[1]);
    } else {
        // Agent tab
        let agent_idx = app.active_tab - 1;
        if agent_idx < installed.len() {
            let agent = installed[agent_idx];
            if let Some(all_idx) = app.agents.iter().position(|a| a.agent_type == agent.agent_type) {
                let sessions = app.agent_sessions.get(all_idx).cloned().unwrap_or_default();
                let rich = app.agent_rich_usage.get(all_idx).cloned();
                draw_agent_tab(f, agent, &sessions, rich.as_ref(), chunks[1]);
            }
        }
    }

    // Status bar
    let refresh_ago = app.last_refresh.elapsed().as_secs();
    let agent_hint = if !installed.is_empty() {
        format!(" 1:Dash {}:Agents 0:KA", installed.len().min(9).to_string())
    } else {
        " 1:Dash 0:KA".to_string()
    };
    let status = format!(
        " q:Quit ←→:Tab{} │ Last refresh: {}s ago",
        agent_hint, refresh_ago,
    );
    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(status_bar, chunks[2]);
}

// ─── Dashboard ───────────────────────────────────────────────────────────────

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let layout = dashboard_layout(area);

    draw_system_status(f, app, layout.system);
    draw_network(f, app, layout.network);
    draw_agents_summary(f, app, layout.agents);
    draw_usage_mini(f, app, layout.usage);
}

fn dashboard_layout(area: Rect) -> DashboardLayout {
    if area.width < MIN_DASHBOARD_WIDTH_FOR_SIDE_BY_SIDE_AGENTS {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(6),
                Constraint::Min(5),
            ])
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);

        DashboardLayout {
            system: top[0],
            network: top[1],
            agents: rows[1],
            usage: rows[2],
        }
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(5)])
            .split(columns[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(0)])
            .split(columns[1]);

        DashboardLayout {
            system: left[0],
            agents: left[1],
            network: right[0],
            usage: right[1],
        }
    }
}

fn draw_system_status(f: &mut Frame, app: &App, area: Rect) {
    let s = &app.system;
    let cpu_pct = s.cpu_usage as u16;
    let ram_pct = s.ram_usage_percent as u16;

    let cpu_color = if cpu_pct > 80 {
        Color::Red
    } else if cpu_pct > 50 {
        Color::Yellow
    } else {
        Color::Green
    };
    let ram_color = if ram_pct > 80 {
        Color::Red
    } else if ram_pct > 50 {
        Color::Yellow
    } else {
        Color::Green
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" System Status ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let cpu_gauge = Gauge::default()
        .gauge_style(Style::default().fg(cpu_color))
        .ratio(cpu_pct as f64 / 100.0);
    f.render_widget(cpu_gauge, rows[0]);

    let cpu_label = Paragraph::new(format!(
        " CPU: {:.1}% ({} cores)",
        s.cpu_usage, s.cpu_cores
    ))
    .style(Style::default().fg(Color::White));
    f.render_widget(cpu_label, rows[1]);

    let ram_gauge = Gauge::default()
        .gauge_style(Style::default().fg(ram_color))
        .ratio(ram_pct as f64 / 100.0);
    f.render_widget(ram_gauge, rows[3]);

    let ram_label = Paragraph::new(format!(
        " RAM: {:.1} / {:.1} GB ({:.1}%)",
        s.ram_used_gb, s.ram_total_gb, s.ram_usage_percent
    ))
    .style(Style::default().fg(Color::White));
    f.render_widget(ram_label, rows[4]);

    let uptime = Paragraph::new(format!(" Uptime: {}", s.uptime_formatted))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(uptime, rows[6]);

    let user = Paragraph::new(format!(" User: {}", s.username))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(user, rows[7]);

    let host = Paragraph::new(format!(" Host: {}", s.hostname))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(host, rows[8]);

    let net = Paragraph::new(format!(
        " Traffic:  ↑ {}  ↓ {}",
        format_rate(s.network_upload_rate),
        format_rate(s.network_download_rate)
    ))
    .style(Style::default().fg(Color::Gray));
    f.render_widget(net, rows[9]);
}

fn draw_network(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Network ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = if let Some(ref info) = app.network {
        vec![
            Line::from(vec![
                Span::raw(" IP:       "),
                Span::styled(&info.ip, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw(" Location: "),
                Span::styled(
                    format!("{}, {}", info.city, info.region),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Country:  "),
                Span::styled(&info.country, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::raw(" ISP:      "),
                Span::styled(&info.org, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::raw(" Timezone: "),
                Span::styled(&info.timezone, Style::default().fg(Color::White)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            " Loading...",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn draw_agents_summary(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Agents ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows: Vec<Row> = app
        .agents
        .iter()
        .map(|a| {
            let status_icon = if a.running { "●" } else if a.installed { "○" } else { "✕" };
            let status_color = if a.running {
                Color::Green
            } else if a.installed {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            let status_text = if a.running {
                let mut parts = Vec::new();
                if a.cli_sessions > 0 {
                    parts.push(format!("{} CLI", a.cli_sessions));
                }
                if a.gui_sessions > 0 {
                    parts.push(format!("{} GUI", a.gui_sessions));
                }
                format!("{} ({})", a.active_sessions, parts.join(", "))
            } else if a.installed {
                "Not Opened".to_string()
            } else {
                "Not Found".to_string()
            };

            let cli_v = a.cli_version.as_deref().unwrap_or("—");
            let gui_v = a.gui_version.as_deref().unwrap_or("—");

            Row::new(vec![
                Cell::from(status_icon).style(Style::default().fg(status_color)),
                Cell::from(a.name.as_str()).style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(cli_v).style(Style::default().fg(Color::Cyan)),
                Cell::from(gui_v).style(Style::default().fg(Color::Magenta)),
                Cell::from(status_text).style(Style::default().fg(status_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Length(14),
        Constraint::Length(22),
        Constraint::Length(16),
        Constraint::Min(14),
    ];

    let table = Table::new(rows, widths).header(
        Row::new(vec!["", "Agent", "CLI Ver", "GUI Ver", "Status"])
            .style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(table, inner);
}

// ─── Agent Tab (per-agent detail view) ───────────────────────────────────────

fn draw_agent_tab(
    f: &mut Frame,
    agent: &AgentInfo,
    sessions: &[Session],
    rich_usage: Option<&AgentUsage>,
    area: Rect,
) {
    // Determine layout based on available data
    let has_rate_limits = rich_usage
        .as_ref()
        .map(|u| u.session_window.is_some() || u.weekly_window.is_some())
        .unwrap_or(false);
    let has_tokens = rich_usage
        .as_ref()
        .map(|u| u.tokens.is_some())
        .unwrap_or(false);

    let header_height = 5;
    let rate_height = if has_rate_limits { 5 } else { 0 };
    let token_height = if has_tokens { 5 } else { 0 };

    let mut constraints = vec![Constraint::Length(header_height)];
    if has_rate_limits {
        constraints.push(Constraint::Length(rate_height));
    }
    if has_tokens {
        constraints.push(Constraint::Length(token_height));
    }
    constraints.push(Constraint::Min(6)); // sessions

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;

    // ── Agent Header ──
    draw_agent_header(f, agent, chunks[idx]);
    idx += 1;

    // ── Rate Limits ──
    if has_rate_limits {
        draw_rate_limits(f, rich_usage.unwrap(), chunks[idx]);
        idx += 1;
    }

    // ── Token Usage ──
    if has_tokens {
        draw_token_usage(f, rich_usage.unwrap(), chunks[idx]);
        idx += 1;
    }

    // ── Sessions table ──
    draw_agent_sessions(f, agent, sessions, chunks[idx]);
}

fn draw_agent_header(f: &mut Frame, agent: &AgentInfo, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", agent.name));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let status_icon = if agent.running { "●" } else { "○" };
    let status_color = if agent.running { Color::Green } else { Color::DarkGray };
    let status_text = if agent.running {
        let mut parts = Vec::new();
        if agent.cli_sessions > 0 {
            parts.push(format!("{} CLI", agent.cli_sessions));
        }
        if agent.gui_sessions > 0 {
            parts.push(format!("{} GUI", agent.gui_sessions));
        }
        format!("Running — {} active ({})", agent.active_sessions, parts.join(", "))
    } else if agent.installed {
        "Installed — Not Running".to_string()
    } else {
        "Not Found".to_string()
    };

    let cli_v = agent.cli_version.as_deref().unwrap_or("—");
    let gui_v = agent.gui_version.as_deref().unwrap_or("—");

    let lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", status_icon), Style::default().fg(status_color)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]),
        Line::from(vec![
            Span::raw(" CLI: "),
            Span::styled(cli_v, Style::default().fg(Color::Cyan)),
            Span::raw("   GUI: "),
            Span::styled(gui_v, Style::default().fg(Color::Magenta)),
        ]),
    ];

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn draw_rate_limits(f: &mut Frame, usage: &AgentUsage, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Rate Limits ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // Session window
    if let Some(ref w) = usage.session_window {
        draw_rate_bar(f, "Session", w.used_percent, w.window_minutes, columns[0]);
    }

    // Weekly window
    if let Some(ref w) = usage.weekly_window {
        draw_rate_bar(f, "Weekly", w.used_percent, w.window_minutes, columns[1]);
    }
}

fn draw_rate_bar(f: &mut Frame, label: &str, used_pct: f64, window_mins: u64, area: Rect) {
    let color = if used_pct >= 90.0 {
        Color::Red
    } else if used_pct >= 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let window_str = if window_mins >= 10080 {
        format!("{}w", window_mins / 10080)
    } else if window_mins >= 1440 {
        format!("{}d", window_mins / 1440)
    } else if window_mins >= 60 {
        format!("{}h", window_mins / 60)
    } else {
        format!("{}m", window_mins)
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ({})", label, window_str),
                Style::default().fg(Color::Gray),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:.1}%", used_pct),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format_bar(used_pct, area.width.saturating_sub(2) as usize),
            Style::default().fg(color),
        )),
    ];

    let p = Paragraph::new(lines);
    f.render_widget(p, area);
}

fn draw_token_usage(f: &mut Frame, usage: &AgentUsage, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Tokens ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tokens = match usage.tokens {
        Some(ref t) => t,
        None => {
            f.render_widget(
                Paragraph::new(" No token data").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return;
        }
    };

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(inner);

    let items = [
        ("Input", tokens.input_tokens, Color::White),
        ("Cache R", tokens.cache_read_tokens, Color::Cyan),
        ("Cache W", tokens.cache_create_tokens, Color::Cyan),
        ("Output", tokens.output_tokens, Color::Green),
        ("Total", tokens.total_tokens, Color::Yellow),
    ];

    for (i, (label, value, color)) in items.iter().enumerate() {
        let lines = vec![
            Line::from(Span::styled(
                *label,
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                format_tokens(*value),
                Style::default().fg(*color).add_modifier(Modifier::BOLD),
            )),
        ];
        f.render_widget(Paragraph::new(lines), columns[i]);
    }
}

fn format_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn draw_agent_sessions(f: &mut Frame, agent: &AgentInfo, sessions: &[Session], area: Rect) {
    let title = format!(" Active Sessions — {} ", agent.name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if sessions.is_empty() {
        let p = Paragraph::new(" No active sessions").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
        return;
    }

    let rows: Vec<Row> = sessions
        .iter()
        .map(|s| {
            let status_style = match s.status.as_str() {
                "busy" => Style::default().fg(Color::Red),
                "idle" | "completed" => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::Yellow),
            };

            let ep = if s.entrypoint.is_empty() { "—" } else { &s.entrypoint };
            let ep_style = match ep {
                "cli" | "sdk-cli" => Style::default().fg(Color::Cyan),
                "vscode" | "exec" => Style::default().fg(Color::Magenta),
                _ => Style::default().fg(Color::DarkGray),
            };

            // Truncate session ID from the front to show the unique suffix
            let sid = if s.id.is_empty() {
                "—".to_string()
            } else {
                truncate_id(&s.id, 22)
            };

            // Working dir: show tail with more space
            let cwd = s.working_dir.as_deref().unwrap_or("—");
            let cwd_display = if cwd.is_empty() {
                "—".to_string()
            } else {
                shorten_path(cwd)
            };

            let status_text = if s.status.is_empty() { "—" } else { &s.status };

            let time = s
                .started_at
                .as_deref()
                .map(relative_time)
                .unwrap_or_default();

            Row::new(vec![
                Cell::from(sid),
                Cell::from(ep).style(ep_style),
                Cell::from(status_text).style(status_style),
                Cell::from(cwd_display),
                Cell::from(time).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    // Responsive widths: session ID fixed, mode+status compact, working dir gets the bulk
    let widths = [
        Constraint::Length(24),  // Session ID
        Constraint::Length(8),   // Mode
        Constraint::Length(10),  // Status
        Constraint::Min(16),    // Working Dir (flexible, gets remaining space)
        Constraint::Length(8),   // Time
    ];

    let table = Table::new(rows, widths).header(
        Row::new(vec!["Session", "Mode", "Status", "Working Dir", "Time"])
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(table, inner);
}

// ─── Usage (legacy full-page, used by dashboard mini) ───────────────────────

fn draw_usage_mini(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Usage ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Collect all usage
    let all_usage: Vec<&UsageStats> = app.agent_usage_5h.iter().collect();
    if all_usage.is_empty() {
        let p = Paragraph::new("No usage data").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
        return;
    }

    let bars: Vec<Bar> = all_usage
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let color = match i % 3 {
                0 => Color::Cyan,
                1 => Color::Green,
                _ => Color::Yellow,
            };
            Bar::default()
                .label(Line::from(u.agent.chars().take(8).collect::<String>()))
                .value(u.total_interactions as u64)
                .style(Style::default().fg(color))
        })
        .collect();

    let barchart = BarChart::default()
        .block(Block::default())
        .data(BarGroup::default().bars(&bars))
        .bar_width(9)
        .bar_gap(1);
    f.render_widget(barchart, inner);
}

// ─── Proxy / VPN ─────────────────────────────────────────────────────────────

fn draw_proxy_tab(f: &mut Frame, info: &ProxyInfo, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // system proxy
            Constraint::Length(5),  // VPN connections
            Constraint::Length(5),  // proxy client
            Constraint::Min(6),    // proxy nodes
        ])
        .split(area);

    draw_proxy_system(f, &info.system_proxy, chunks[0]);
    draw_proxy_vpn(f, &info.vpn_connections, chunks[1]);
    draw_proxy_client(f, &info.active_client, chunks[2]);
    draw_proxy_nodes(f, &info.proxy_nodes, chunks[3]);
}

fn draw_proxy_system(f: &mut Frame, proxy: &crate::models::types::SystemProxy, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" System Proxy — {} ", proxy.active_service));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    draw_proxy_entry(f, "HTTP", &proxy.http, columns[0]);
    draw_proxy_entry(f, "HTTPS", &proxy.https, columns[1]);
    draw_proxy_entry_socks(f, "SOCKS", &proxy.socks, columns[2]);

    // PAC
    let pac_lines = if let Some(ref url) = proxy.pac {
        vec![
            Line::from(Span::styled(" PAC", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(
                truncate_str(url, 22),
                Style::default().fg(Color::Cyan),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled(" PAC", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" Off", Style::default().fg(Color::DarkGray))),
        ]
    };
    let pac_block = Block::default().borders(Borders::RIGHT);
    let pac_inner = pac_block.inner(columns[3]);
    f.render_widget(pac_block, columns[3]);
    f.render_widget(Paragraph::new(pac_lines), pac_inner);
}

fn draw_proxy_entry(f: &mut Frame, label: &str, entry: &crate::models::types::ProxyEntry, area: Rect) {
    let lines = if entry.enabled {
        vec![
            Line::from(Span::styled(
                format!(" {}", label),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(" {}:{}", entry.server, entry.port),
                Style::default().fg(Color::Green),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                format!(" {}", label),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(" Off", Style::default().fg(Color::DarkGray))),
        ]
    };
    let block = Block::default().borders(Borders::RIGHT);
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_proxy_entry_socks(f: &mut Frame, label: &str, entry: &crate::models::types::ProxyEntry, area: Rect) {
    draw_proxy_entry(f, label, entry, area);
}

fn draw_proxy_vpn(f: &mut Frame, vpns: &[crate::models::types::VpnConnection], area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" VPN ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if vpns.is_empty() {
        f.render_widget(
            Paragraph::new(" No VPN connections detected").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let rows: Vec<Row> = vpns
        .iter()
        .map(|v| {
            let status_icon = if v.connected { "●" } else { "○" };
            let status_color = if v.connected { Color::Green } else { Color::DarkGray };
            let state_text = if v.connected { "Connected" } else { "Disconnected" };

            Row::new(vec![
                Cell::from(status_icon).style(Style::default().fg(status_color)),
                Cell::from(v.name.as_str()).style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(v.vpn_type.as_str()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(state_text).style(Style::default().fg(status_color)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(16),
            Constraint::Length(10),
            Constraint::Length(14),
        ],
    );
    f.render_widget(table, inner);
}

fn draw_proxy_client(f: &mut Frame, client: &Option<crate::models::types::ProxyClient>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Proxy Client ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(c) = client {
        let mut lines = vec![Line::from(vec![
            Span::styled(format!(" {} ", c.name), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(c.client_type.to_uppercase(), Style::default().fg(Color::DarkGray)),
        ])];
        let mut detail_parts = vec![format!("port: {}", c.api_port)];
        if let Some(ref v) = c.version {
            detail_parts.push(format!("v{}", v));
        }
        if let Some(ref m) = c.mode {
            detail_parts.push(format!("mode: {}", m));
        }
        lines.push(Line::from(Span::styled(
            format!(" {}", detail_parts.join("  │  ")),
            Style::default().fg(Color::Gray),
        )));
        f.render_widget(Paragraph::new(lines), inner);
    } else {
        f.render_widget(
            Paragraph::new(" No proxy client detected").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
    }
}

fn draw_proxy_nodes(f: &mut Frame, nodes: &[crate::models::types::ProxyNode], area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Proxy Nodes ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if nodes.is_empty() {
        f.render_widget(
            Paragraph::new(" No proxy nodes available").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let rows: Vec<Row> = nodes
        .iter()
        .map(|n| {
            let delay_str = n
                .delay
                .map(|d| {
                    if d >= 1000 {
                        format!("{}.{:01}s", d / 1000, (d % 1000) / 100)
                    } else {
                        format!("{}ms", d)
                    }
                })
                .unwrap_or_else(|| "—".to_string());

            let delay_color = match n.delay {
                Some(d) if d <= 100 => Color::Green,
                Some(d) if d <= 300 => Color::Yellow,
                Some(_) => Color::Red,
                None => Color::DarkGray,
            };

            Row::new(vec![
                Cell::from(n.name.as_str()).style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(n.node_type.as_str()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(n.selected.as_str()).style(Style::default().fg(Color::Cyan)),
                Cell::from(delay_str).style(Style::default().fg(delay_color)),
                Cell::from(format!("{} nodes", n.available_nodes.len()))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(10),
            Constraint::Min(18),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["Group", "Type", "Selected", "Delay", "Nodes"])
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(table, inner);
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let tail: String = s.chars().skip(char_count - max_chars + 1).collect();
        format!("…{}", tail)
    }
}

// ─── Keep-Alive ──────────────────────────────────────────────────────────────

fn draw_keepalive(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(5)])
        .split(area);

    // Status
    let status_block = Block::default()
        .borders(Borders::ALL)
        .title(" Keep-Alive Status ");

    let status_inner = status_block.inner(chunks[0]);
    f.render_widget(status_block, chunks[0]);

    let status_lines = if app.keepalive.active {
        vec![
            Line::from(vec![
                Span::raw(" Status:  "),
                Span::styled(
                    "ACTIVE",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Mode:    "),
                Span::styled(
                    app.keepalive.mode.as_deref().unwrap_or("?"),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Started: "),
                Span::styled(
                    app.keepalive.started_at.as_deref().unwrap_or("?"),
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Press Space to stop",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::raw(" Status: "),
                Span::styled("OFF", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Press Space to start (1h default)",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };
    let status_p = Paragraph::new(status_lines);
    f.render_widget(status_p, status_inner);

    // Controls
    let controls_block = Block::default()
        .borders(Borders::ALL)
        .title(" Controls ");

    let controls_inner = controls_block.inner(chunks[1]);
    f.render_widget(controls_block, chunks[1]);

    let modes = [
        ("a", "30 min", "30m"),
        ("s", "1 hour", "1h"),
        ("d", "3 hours", "3h"),
        ("f", "Forever", "forever"),
        ("Space", "Toggle", ""),
    ];

    let rows: Vec<Row> = modes
        .iter()
        .map(|(key, label, mode)| {
            let is_active = app.keepalive.active
                && !mode.is_empty()
                && app.keepalive.mode.as_deref() == Some(mode);

            let key_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let label_style = if is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let indicator = if is_active { " ◀ active" } else { "" };

            Row::new(vec![
                Cell::from(format!(" [{}] ", key)).style(key_style),
                Cell::from(format!(" {} ", label)).style(label_style),
                Cell::from(indicator).style(Style::default().fg(Color::Green)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Min(10),
        ],
    );
    f.render_widget(table, controls_inner);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Truncate a session ID to `max_len` chars, keeping the right (unique) part
fn truncate_id(id: &str, max_len: usize) -> String {
    let char_count = id.chars().count();
    if char_count <= max_len {
        id.to_string()
    } else {
        let skip = char_count - max_len + 1; // +1 for the '…' prefix
        let tail: String = id.chars().skip(skip).collect();
        format!("…{}", tail)
    }
}

/// Shorten a path by replacing the middle with …, keeping the tail.
/// Uses char-based operations to avoid panicking on multi-byte UTF-8.
fn shorten_path(path: &str) -> String {
    let char_count = path.chars().count();
    if char_count <= 32 {
        return path.to_string();
    }
    // Keep the last ~28 chars
    let tail_chars = 28;
    if char_count <= tail_chars + 2 {
        return path.to_string();
    }
    let tail: String = path.chars().skip(char_count - tail_chars).collect();
    // Find the first / in tail to keep it clean
    let offset = tail.find('/').unwrap_or(0);
    // offset is a byte index into `tail` which is ASCII-ish (paths), safe to slice
    format!("…{}", &tail[offset..])
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

fn relative_time(iso: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let diff = chrono::Utc::now() - dt.with_timezone(&chrono::Utc);
    let mins = diff.num_minutes();
    if mins < 1 {
        "now".to_string()
    } else if mins < 60 {
        format!("{}m", mins)
    } else if mins < 1440 {
        format!("{}h", mins / 60)
    } else {
        format!("{}d", mins / 1440)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_layout_gives_agents_full_width_on_narrow_terminals() {
        let area = Rect::new(0, 0, 80, 20);
        let layout = dashboard_layout(area);

        assert_eq!(layout.agents.x, area.x);
        assert_eq!(layout.agents.width, area.width);
    }

    #[test]
    fn dashboard_layout_keeps_side_by_side_panels_on_wide_terminals() {
        let area = Rect::new(0, 0, 180, 24);
        let layout = dashboard_layout(area);

        assert_eq!(layout.agents.x, layout.system.x);
        assert!(layout.agents.width < area.width);
        assert_eq!(layout.usage.x, layout.network.x);
    }

    #[test]
    fn truncate_id_short_ids_unchanged() {
        assert_eq!(truncate_id("abc", 10), "abc");
        assert_eq!(truncate_id("1234567890", 10), "1234567890");
    }

    #[test]
    fn truncate_id_long_ids_keep_right() {
        let result = truncate_id("abcdefghijklmnop", 8);
        assert!(result.starts_with("…"));
        // Use chars().count() since '…' is multi-byte UTF-8
        assert!(result.chars().count() <= 8, "expected <= 8 chars, got {}: {}", result.chars().count(), result);
        assert!(result.ends_with("mnop"));
    }

    #[test]
    fn shorten_path_short_unchanged() {
        assert_eq!(shorten_path("/tmp/test"), "/tmp/test");
    }

    #[test]
    fn shorten_path_long_path_truncated() {
        let path = "/Users/developer/projects/my-app/src/components/very-long-name.tsx";
        let result = shorten_path(path);
        assert!(result.starts_with("…"));
        assert!(result.len() < path.len());
    }
}
