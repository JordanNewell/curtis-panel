mod action;
mod cli;
mod commands;
mod config;
mod state;

use commands::AppState;
use tauri::Manager;

fn main() -> anyhow::Result<()> {
    cli::run()
}

pub fn run_app() -> anyhow::Result<()> {
    let config_path = config::default_config_path();
    let config = if config_path.exists() {
        config::load_from_path(&config_path)?
    } else {
        anyhow::bail!(
            "no config found at {}. Run `openpanel check <path>` to validate a config first.",
            config_path.display()
        );
    };

    tauri::Builder::default()
        .manage(AppState::new(config))
        .invoke_handler(tauri::generate_handler![get_panel, trigger_action])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}

#[tauri::command]
fn get_panel(state: tauri::State<'_, AppState>) -> commands::PanelSnapshot {
    state.panel_snapshot()
}

#[tauri::command]
fn trigger_action(
    button_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<commands::TriggerResult, String> {
    state.trigger(&button_id).map_err(|e| e.to_string())
}
