use crate::core_modules::{agents, keepalive, network, system};
use crate::models::types::{AgentInfo, KeepAliveStatus, NetworkInfo, Session, SystemStatus, UsageStats};
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

const TAB_NAMES: &[&str] = &["Dashboard", "Sessions", "Usage", "Keep-Alive"];

struct App {
    system: SystemStatus,
    network: Option<NetworkInfo>,
    agents: Vec<AgentInfo>,
    sessions: Vec<Session>,
    usage: Vec<UsageStats>,
    keepalive: KeepAliveStatus,
    active_tab: usize,
    usage_window: String,
    last_refresh: Instant,
    network_loaded: bool,
}

impl App {
    fn new() -> Self {
        Self {
            system: system::get_system_status(),
            network: None,
            agents: agents::detect_all_agents(),
            sessions: Vec::new(),
            usage: Vec::new(),
            keepalive: keepalive::get_keepalive_status(),
            active_tab: 0,
            usage_window: "5h".to_string(),
            last_refresh: Instant::now(),
            network_loaded: false,
        }
    }

    fn refresh(&mut self) {
        self.system = system::get_system_status();
        self.agents = agents::detect_all_agents();
        self.keepalive = keepalive::get_keepalive_status();

        // Load network once
        if !self.network_loaded {
            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Ok(info) = rt.block_on(network::get_network_info()) {
                self.network = Some(info);
                self.network_loaded = true;
            }
        }

        // Sessions
        self.sessions = Vec::new();
        for agent in &self.agents {
            let mut s = agents::get_sessions(&agent.agent_type);
            self.sessions.append(&mut s);
        }

        // Usage
        self.usage = self
            .agents
            .iter()
            .map(|a| agents::get_usage(&a.agent_type, &self.usage_window))
            .collect();

        self.last_refresh = Instant::now();
    }

    fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % TAB_NAMES.len();
    }

    fn prev_tab(&mut self) {
        self.active_tab = if self.active_tab == 0 {
            TAB_NAMES.len() - 1
        } else {
            self.active_tab - 1
        };
    }

    fn cycle_usage_window(&mut self) {
        self.usage_window = match self.usage_window.as_str() {
            "5h" => "1w".to_string(),
            "1w" => "1m".to_string(),
            _ => "5h".to_string(),
        };
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
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Right | KeyCode::Tab => app.next_tab(),
                        KeyCode::Left | KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Char('1') => app.active_tab = 0,
                        KeyCode::Char('2') => app.active_tab = 1,
                        KeyCode::Char('3') => app.active_tab = 2,
                        KeyCode::Char('4') => app.active_tab = 3,
                        KeyCode::Char('r') => app.refresh(),
                        KeyCode::Char('w') => {
                            app.cycle_usage_window();
                            app.refresh();
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
    let titles: Vec<Line> = TAB_NAMES
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
    match app.active_tab {
        0 => draw_dashboard(f, app, chunks[1]),
        1 => draw_sessions(f, app, chunks[1]),
        2 => draw_usage(f, app, chunks[1]),
        3 => draw_keepalive(f, app, chunks[1]),
        _ => {}
    }

    // Status bar
    let refresh_ago = app.last_refresh.elapsed().as_secs();
    let status = format!(
        " q:Quit  ←→:Tab  r:Refresh  w:Window  Space:Toggle  │  Last refresh: {}s ago  Window: {}",
        refresh_ago, app.usage_window
    );
    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(status_bar, chunks[2]);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(5)])
        .split(h_chunks[0]);

    draw_system_status(f, app, left_chunks[0]);
    draw_agents(f, app, left_chunks[1]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(h_chunks[1]);

    draw_network(f, app, right_chunks[0]);
    draw_usage_mini(f, app, right_chunks[1]);
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
            Constraint::Length(1), // CPU gauge
            Constraint::Length(1), // CPU label
            Constraint::Length(1), // spacer
            Constraint::Length(1), // RAM gauge
            Constraint::Length(1), // RAM label
            Constraint::Length(1), // spacer
            Constraint::Length(1), // uptime
            Constraint::Length(1), // user
            Constraint::Length(1), // host
            Constraint::Length(1), // network
        ])
        .split(inner);

    // CPU gauge
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

    // RAM gauge
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

fn draw_agents(f: &mut Frame, app: &App, area: Rect) {
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

fn draw_sessions(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Active Sessions ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.sessions.is_empty() {
        let p = Paragraph::new("No active sessions").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
        return;
    }

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .map(|s| {
            let status_style = match s.status.as_str() {
                "busy" => Style::default().fg(Color::Red),
                "idle" | "completed" => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::Yellow),
            };
            let cwd = s.working_dir.as_deref().unwrap_or("—");
            let cwd_short = if cwd.len() > 30 {
                format!("…{}", &cwd[cwd.len() - 29..])
            } else {
                cwd.to_string()
            };
            let time = s
                .started_at
                .as_deref()
                .map(relative_time)
                .unwrap_or_default();

            let ep = if s.entrypoint.is_empty() { "—" } else { &s.entrypoint };
            let ep_style = match ep {
                "cli" | "sdk-cli" => Style::default().fg(Color::Cyan),
                "vscode" | "exec" => Style::default().fg(Color::Magenta),
                _ => Style::default().fg(Color::DarkGray),
            };

            Row::new(vec![
                Cell::from(s.agent.as_str()).style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(s.id.chars().take(18).collect::<String>()),
                Cell::from(ep).style(ep_style),
                Cell::from(s.status.as_str()).style(status_style),
                Cell::from(cwd_short),
                Cell::from(time).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(20),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Min(18),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths).header(
        Row::new(vec!["Agent", "Session", "Mode", "Status", "Working Dir", "Time"])
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(table, inner);
}

fn draw_usage(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Window selector
    let window_label = format!(" Window: {} (press 'w' to cycle) ", app.usage_window);
    let tabs = Tabs::new(vec![
        Line::from(" 5 Hours "),
        Line::from(" 1 Week "),
        Line::from(" 1 Month "),
    ])
    .block(Block::default().borders(Borders::ALL).title(window_label))
    .select(match app.usage_window.as_str() {
        "5h" => 0,
        "1w" => 1,
        "1m" => 2,
        _ => 0,
    })
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().fg(Color::Cyan));
    f.render_widget(tabs, chunks[0]);

    // Usage content
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Usage Statistics ");

    let inner = block.inner(chunks[1]);
    f.render_widget(block, chunks[1]);

    if app.usage.is_empty() {
        let p = Paragraph::new("No usage data").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
        return;
    }

    // Table view
    let rows: Vec<Row> = app
        .usage
        .iter()
        .map(|u| {
            Row::new(vec![
                Cell::from(u.agent.as_str()).style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(u.total_sessions.to_string()).style(Style::default().fg(Color::Cyan)),
                Cell::from(format_number(u.total_interactions)).style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Min(14),
        ],
    )
    .header(
        Row::new(vec!["Agent", "Sessions", "Interactions"])
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(table, inner);
}

fn draw_usage_mini(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Usage ({}) ", app.usage_window));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.usage.is_empty() {
        let p = Paragraph::new("No usage data").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, inner);
        return;
    }

    let bars: Vec<Bar> = app
        .usage
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

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
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
