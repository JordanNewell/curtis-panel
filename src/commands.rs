use std::sync::Arc;
use anyhow::Result;
use parking_lot::RwLock;
use crate::config::Config;
use crate::state::{ButtonState, StateStore};
use crate::action;

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub state: StateStore,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let state = StateStore::new();
        for btn in &config.panel.buttons {
            state.init_button(&btn.id);
        }
        Self {
            config: Arc::new(RwLock::new(config)),
            state,
        }
    }

    pub fn panel_snapshot(&self) -> PanelSnapshot {
        let cfg = self.config.read();
        PanelSnapshot {
            panel: cfg.panel.clone(),
            states: self.state.get_all(),
        }
    }

    pub fn trigger(&self, button_id: &str) -> Result<TriggerResult> {
        let cfg = self.config.read();
        let button = cfg.panel.buttons.iter().find(|b| b.id == button_id)
            .ok_or_else(|| anyhow::anyhow!("button '{}' not found", button_id))?;

        self.state.set_state(button_id, ButtonState::Pressed);
        let result = action::execute(&button.action)?;
        let new_state = if result.success { ButtonState::Active } else { ButtonState::Error };
        self.state.set_state(button_id, new_state);
        Ok(TriggerResult {
            success: result.success,
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
        })
    }
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct PanelSnapshot {
    pub panel: crate::config::Panel,
    pub states: Vec<crate::state::ButtonStatus>,
}

#[derive(Debug, serde::Serialize)]
pub struct TriggerResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[tauri::command]
pub fn get_panel(state: tauri::State<'_, AppState>) -> PanelSnapshot {
    state.panel_snapshot()
}

#[tauri::command]
pub fn trigger_action(
    button_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<TriggerResult, String> {
    state.trigger(&button_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        serde_yaml::from_str(r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons:
    - { id: btn1, shape: square, slot: [1, 1], action: { type: shell, command: echo, args: ["hi"] } }
"#).unwrap()
    }

    #[test]
    fn snapshot_returns_panel_with_states() {
        let state = AppState::new(test_config());
        let snap = state.panel_snapshot();
        assert_eq!(snap.panel.id, "test");
        assert_eq!(snap.states.len(), 1);
        assert_eq!(snap.states[0].id, "btn1");
    }

    #[test]
    fn trigger_runs_action_and_updates_state() {
        let state = AppState::new(test_config());
        let result = state.trigger("btn1").unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "hi");
        let snap = state.panel_snapshot();
        assert_eq!(snap.states[0].state, ButtonState::Active);
    }

    #[test]
    fn trigger_unknown_button_returns_error() {
        let state = AppState::new(test_config());
        assert!(state.trigger("nonexistent").is_err());
    }
}
