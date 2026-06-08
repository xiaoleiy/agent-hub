mod cli;
mod commands;
mod core_modules;
mod models;
mod tui;

use std::env;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

/// Show or hide the main window.
fn toggle_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

/// Anchor the window just below the tray icon so it reads as a menu-bar popover.
fn position_under_tray(win: &tauri::WebviewWindow, rect: tauri::Rect) {
    use tauri::{PhysicalPosition, Position, Size};

    let scale = win.scale_factor().unwrap_or(1.0);
    // Tray icon top-left + size, normalized to physical pixels.
    let (icon_x, icon_y, icon_w, icon_h) = match (rect.position, rect.size) {
        (Position::Physical(p), Size::Physical(s)) => {
            (p.x as f64, p.y as f64, s.width as f64, s.height as f64)
        }
        (Position::Logical(p), Size::Logical(s)) => {
            (p.x * scale, p.y * scale, s.width * scale, s.height * scale)
        }
        _ => return,
    };
    let win_w = win.outer_size().map(|s| s.width as f64).unwrap_or(480.0);
    // Center the window under the icon; sit just below the menu bar.
    let x = icon_x + icon_w / 2.0 - win_w / 2.0;
    let y = icon_y + icon_h + 2.0;
    let _ = win.set_position(Position::Physical(PhysicalPosition {
        x: x.max(8.0) as i32,
        y: y as i32,
    }));
}

/// Entry point: routes to CLI mode or GUI mode based on arguments.
/// No args or `gui` → launch the menu-bar (tray) GUI.
/// Any other args → run CLI command and exit.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = env::args().collect();

    // If there are CLI arguments (beyond the binary name), run in CLI mode
    if args.len() > 1 && args[1] != "gui" {
        cli::run_cli(&args[1..]);
        return;
    }

    // GUI mode — a macOS menu-bar app (no Dock icon, no standalone window).
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar app: hide the Dock icon so it lives only in the tray.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Tray menu: a Quit item plus Show/Hide.
            let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Agent Hub", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_hide, &quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false) // left-click toggles; right-click opens menu
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => app.exit(0),
                    "show_hide" => toggle_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                position_under_tray(&win, rect);
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Closing the window hides it back to the tray (doesn't quit).
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
            commands::agents::get_agent_rich_usage,
            commands::proxy::get_proxy_info,
            commands::keepalive::get_keepalive_status,
            commands::keepalive::set_keepalive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
