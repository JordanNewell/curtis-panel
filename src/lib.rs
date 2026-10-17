pub mod action;
pub mod apps;
pub mod cli;
pub mod commands;
pub mod config;
pub mod state;

use commands::AppState;
use tauri::Manager;

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
        .invoke_handler(tauri::generate_handler![commands::get_panel, commands::trigger_action])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
