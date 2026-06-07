mod cli;
mod commands;
mod core_modules;
mod models;
mod tui;

use std::env;

/// Entry point: routes to CLI mode or GUI mode based on arguments.
/// No args or `gui` → launch Tauri GUI window.
/// Any other args → run CLI command and exit.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = env::args().collect();

    // If there are CLI arguments (beyond the binary name), run in CLI mode
    if args.len() > 1 && args[1] != "gui" {
        cli::run_cli(&args[1..]);
        return;
    }

    // GUI mode — launch Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::system::get_system_status,
            commands::network::get_network_info,
            commands::agents::get_agents,
            commands::agents::get_agent_sessions,
            commands::agents::get_agent_usage,
            commands::keepalive::get_keepalive_status,
            commands::keepalive::set_keepalive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
