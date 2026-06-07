mod cli;
mod commands;
mod core_modules;
mod models;
mod tui;

use std::env;
use tauri::Manager;

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
        .setup(|app| {
            // Set up tray icon click handler: toggle main window visibility
            if let Some(tray) = app.tray_by_id("main") {
                let app_handle = app.handle().clone();
                tray.on_tray_icon_event(move |_tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });
            }

            // Handle window close: hide to tray instead of quitting
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Prevent the default close, hide the window instead
                        api.prevent_close();
                        if let Some(win) = app_handle.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_system_status,
            commands::network::get_network_info,
            commands::agents::get_agents,
            commands::agents::get_agent_sessions,
            commands::agents::get_agent_usage,
            commands::proxy::get_proxy_info,
            commands::keepalive::get_keepalive_status,
            commands::keepalive::set_keepalive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
