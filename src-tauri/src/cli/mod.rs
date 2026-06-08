pub mod handlers;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "agent-hub",
    version,
    about = "Manage AI coding agents on your machine"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show system status (CPU, RAM, uptime, network)
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show network information (public IP from ipinfo.io)
    Network {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List detected agents and their status
    Agents {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show active sessions
    Sessions {
        /// Filter by agent name (claude, cursor, codex)
        #[arg(long)]
        agent: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show usage statistics
    Usage {
        /// Time window: 5h, 1w, 1m
        #[arg(long, default_value = "5h")]
        window: String,
        /// Filter by agent name
        #[arg(long)]
        agent: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage keep-alive (prevent sleep)
    Keepalive {
        /// Mode: off, 30m, 1h, 3h, forever, or --status
        mode: Option<String>,
        /// Show current keep-alive status
        #[arg(long)]
        status: bool,
    },

    /// Launch the interactive terminal UI (TUI dashboard)
    Tui,

    /// Launch the GUI window
    Gui,
}

pub fn run_cli(args: &[String]) {
    let cli = Cli::parse_from(std::iter::once("agent-hub".to_string()).chain(args.iter().cloned()));

    match cli.command {
        Some(Commands::Status { json }) => handlers::status(json),
        Some(Commands::Network { json }) => handlers::network(json),
        Some(Commands::Agents { json }) => handlers::agents(json),
        Some(Commands::Sessions { agent, json }) => handlers::sessions(agent, json),
        Some(Commands::Usage {
            window,
            agent,
            json,
        }) => handlers::usage(&window, agent, json),
        Some(Commands::Keepalive { mode, status }) => handlers::keepalive(mode, status),
        Some(Commands::Tui) => {
            crate::tui::run_tui();
        }
        Some(Commands::Gui) => {
            // GUI mode — this shouldn't normally be reached since lib.rs
            // handles the "gui" arg, but just in case:
            println!("Launching GUI...");
        }
        None => {
            // No subcommand — also launch GUI (handled by lib.rs, but just in case)
            println!("Launching GUI...");
        }
    }
}
