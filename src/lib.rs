pub mod action;
pub mod apps;
pub mod cli;
pub mod commands;
pub mod config;
pub mod dock;
pub mod geometry;
pub mod shell;
pub mod state;

pub fn run_app() -> anyhow::Result<()> {
    use crate::config::DockMode;
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
        Manager,
    };
    use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

    let config_path = config::default_config_path();
    let config = if config_path.exists() {
        config::load_from_path(&config_path)?
    } else {
        anyhow::bail!(
            "no config found at {}. Run `curtis-panel check <path>` to validate a config first.",
            config_path.display()
        );
    };

    let hotkey: Shortcut = config.dock.hotkey.parse().map_err(|e| {
        anyhow::anyhow!("invalid hotkey '{}' in config: {e}", config.dock.hotkey)
    })?;
    let peek_enabled = config.dock.peek;

    let app_state = commands::AppState::new(config);
    app_state.reload_apps(&config_path);

    // Restore persisted dock state (edge overrides + drag sizes).
    if let Ok(Some(mut saved)) = crate::dock::DockState::load(&config::default_dock_state_path()) {
        // Always start hidden — the hotkey or peek summons the panel.
        saved.shown = false;
        *app_state.dock.write() = saved;
    }

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([hotkey])?
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Err(e) = crate::shell::toggle_dock(app) {
                            tracing::error!("dock toggle failed: {e}");
                        }
                    }
                })
                .build(),
        )
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_panel,
            commands::trigger_action,
            commands::get_dock,
            commands::set_dock_size
        ])
        .setup(move |app| {
            crate::shell::position_dock(app.handle())?;
            if peek_enabled {
                crate::shell::position_peek(app.handle())?;
            }
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let open = MenuItem::with_id(app, "open", "Open panel", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("curtis-panel")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        let _ = crate::shell::toggle_dock(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let app = window.app_handle();
            let mode = app
                .state::<crate::commands::AppState>()
                .config
                .read()
                .dock
                .mode;
            match event {
                // Click-away dismissal only in dock mode — a popped-out panel
                // is a normal window and survives focus loss.
                tauri::WindowEvent::Focused(false) if mode == DockMode::Dock => {
                    if let Err(e) = crate::shell::hide_dock(app) {
                        tracing::error!("dock hide failed: {e}");
                    }
                }
                tauri::WindowEvent::Resized(_) if mode == DockMode::Dock => {
                    // Drag-resize: persist the new cross dimension per edge.
                    crate::shell::note_resize(window);
                }
                tauri::WindowEvent::Moved(_) if mode == DockMode::Float => {
                    crate::shell::note_move(window);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
