use std::sync::Arc;
use anyhow::Result;
use parking_lot::RwLock;
use crate::action;
use crate::apps::{load_apps, AppSizeHint};
use crate::config::{apps_dir_for, Config, DockSize};
use crate::dock::{DockState, SizeChoice};
use crate::state::{ButtonState, StateStore};

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub state: StateStore,
    pub dock: RwLock<DockState>,
    pub apps: RwLock<Vec<crate::apps::LoadedApp>>,
    pub app_errors: RwLock<Vec<crate::apps::AppLoadError>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let state = StateStore::new();
        for btn in &config.panel.buttons {
            state.init_button(&btn.id);
        }
        let dock = DockState::new(config.dock.edge, config.dock.size);
        Self {
            config: Arc::new(RwLock::new(config)),
            state,
            dock: RwLock::new(dock),
            apps: RwLock::new(Vec::new()),
            app_errors: RwLock::new(Vec::new()),
        }
    }

    /// Scan the apps dir declared by the config. Call after config load /
    /// reload; invalid manifests are logged, never fatal.
    pub fn reload_apps(&self, config_path: &std::path::Path) {
        let cfg = self.config.read();
        let apps_dir = apps_dir_for(&cfg, config_path);
        drop(cfg);
        let (loaded, errors) = load_apps(&apps_dir);
        for e in &errors {
            tracing::warn!("skipping app dir {}: {}", e.dir.display(), e.message);
        }
        *self.apps.write() = loaded;
        *self.app_errors.write() = errors;
    }

    pub fn set_dock_size(&self, size: DockSize) {
        self.dock.write().set_size(size);
    }

    pub fn dock_snapshot(&self) -> DockSnapshot {
        let dock = self.dock.read();
        let cfg = self.config.read();
        DockSnapshot {
            mode: cfg.dock.mode,
            edge: dock.edge,
            hotkey: cfg.dock.hotkey.clone(),
            shown: dock.shown,
            size: dock.size,
            effective: dock.effective_size(),
            motion: cfg.dock.motion,
            peek: cfg.dock.peek,
            apps: self
                .apps
                .read()
                .iter()
                .map(|a| AppInfo {
                    name: a.manifest.name.clone(),
                    icon: a.manifest.icon.clone(),
                    url: a.manifest.url.clone(),
                    entry: a.manifest.entry.clone().map(|e| {
                        a.dir.join(e).to_string_lossy().to_string()
                    }),
                    size: a.manifest.size,
                })
                .collect(),
            errors: self
                .app_errors
                .read()
                .iter()
                .map(|e| format!("{}: {}", e.dir.display(), e.message))
                .collect(),
        }
    }

    // test hook — not used in production paths
    #[cfg(test)]
    pub fn set_apps_for_test(&self, manifests: Vec<crate::apps::AppManifest>) {
        *self.apps.write() = manifests
            .into_iter()
            .map(|manifest| crate::apps::LoadedApp {
                dir: std::path::PathBuf::from("/dev/null"),
                manifest,
            })
            .collect();
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

#[derive(Debug, serde::Serialize, Clone)]
pub struct AppInfo {
    pub name: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub entry: Option<String>,
    pub size: AppSizeHint,
}

/// Full dock state for the frontend (`get_dock` IPC contract).
///
/// - `mode`/`hotkey`/`peek`/`motion` mirror the static config; `hotkey` is
///   the toggle binding to hint in the UI (e.g. `"alt+c"`).
/// - `edge`/`shown`/`size` are runtime dock state; `size` is the active
///   *named preset* and may lag behind the user's drag.
/// - `effective` is the authoritative window size (`{"preset":"folio"}` or
///   `{"custom":{"width":500,"height":800}}`) — always render from this,
///   never from `size` alone.
/// - `entry` in each app is the manifest path resolved against the app dir.
/// - `errors` lists app dirs skipped at load (`"<dir>: <reason>"`), never
///   fatal — surface as a badge, not a blocker.
#[derive(Debug, serde::Serialize, Clone)]
pub struct DockSnapshot {
    pub mode: crate::config::DockMode,
    pub edge: crate::config::DockEdge,
    pub hotkey: String,
    pub shown: bool,
    pub size: DockSize,
    pub effective: SizeChoice,
    pub motion: crate::config::DockMotion,
    pub peek: bool,
    pub apps: Vec<AppInfo>,
    pub errors: Vec<String>,
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

#[tauri::command]
pub fn get_dock(state: tauri::State<'_, AppState>) -> DockSnapshot {
    state.dock_snapshot()
}

#[tauri::command]
pub fn set_dock_size(
    size: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let size = match size.as_str() {
        "ledger" => DockSize::Ledger,
        "folio" => DockSize::Folio,
        "full" => DockSize::Full,
        other => return Err(format!("unknown size '{other}'")),
    };
    state.set_dock_size(size);
    Ok(())
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

    use crate::apps::AppManifest;
    use crate::config::{DockEdge, DockMode, DockSize};
    use crate::dock::{PersistedSize, SizeChoice};
    use assert_fs::prelude::*;

    fn test_state_with_dock() -> AppState {
        let config = test_config();
        AppState::new(config)
    }

    #[test]
    fn dock_snapshot_lists_loaded_apps() {
        let state = test_state_with_dock();
        state.set_apps_for_test(vec![
            AppManifest {
                name: "Curtis Chat".into(),
                icon: None,
                url: Some("https://app.curtisai.tech".into()),
                entry: None,
                size: crate::apps::AppSizeHint::Full,
                hotkey: None,
            },
            AppManifest {
                name: "Local Tool".into(),
                icon: None,
                url: None,
                entry: Some("ui/index.html".into()),
                size: crate::apps::AppSizeHint::default(),
                hotkey: None,
            },
        ]);
        let snap = state.dock_snapshot();
        assert_eq!(snap.edge, DockEdge::Right);
        assert_eq!(snap.mode, DockMode::Dock);
        assert_eq!(snap.hotkey, "alt+c");
        assert!(matches!(snap.motion, crate::config::DockMotion::Drawer));
        assert_eq!(snap.apps.len(), 2);
        assert_eq!(snap.apps[0].name, "Curtis Chat");
        assert_eq!(snap.apps[1].name, "Local Tool");
        let entry = snap.apps[1].entry.as_deref().unwrap();
        assert!(entry.ends_with("ui/index.html"), "entry was {entry}");
        assert!(snap.errors.is_empty());
    }

    #[test]
    fn set_dock_size_updates_state_and_clears_custom() {
        let state = test_state_with_dock();
        state.dock.write().set_custom(500, 800);
        state.set_dock_size(DockSize::Ledger);
        let snap = state.dock_snapshot();
        assert_eq!(snap.size, DockSize::Ledger);
        assert_eq!(snap.effective, SizeChoice::Preset(DockSize::Ledger));
    }

    #[test]
    fn dock_snapshot_reports_effective_custom_size() {
        let state = test_state_with_dock();
        state.dock.write().set_custom(500, 800);
        let snap = state.dock_snapshot();
        assert_eq!(
            snap.effective,
            SizeChoice::Custom(PersistedSize { width: 500, height: 800 })
        );
    }

    #[test]
    fn dock_snapshot_serializes_self_describing_shape() {
        let state = test_state_with_dock();
        state.dock.write().set_custom(500, 800);
        let json = serde_json::to_value(state.dock_snapshot()).unwrap();
        assert_eq!(
            json["effective"],
            serde_json::json!({ "custom": { "width": 500, "height": 800 } })
        );

        let state = test_state_with_dock();
        let json = serde_json::to_value(state.dock_snapshot()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "mode": "dock",
                "edge": "right",
                "hotkey": "alt+c",
                "shown": false,
                "size": "folio",
                "effective": { "preset": "folio" },
                "motion": "drawer",
                "peek": true,
                "apps": [],
                "errors": []
            })
        );
    }

    #[test]
    fn reload_apps_loads_valid_and_surfaces_errors() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("config.yaml").write_str(r#"
panel:
  id: test
  name: Test
  grid: { columns: 1, rows: 1 }
  buttons: []
apps_dir: apps
"#).unwrap();
        temp.child("apps/good/app.curt.yaml")
            .write_str("name: Good App\nurl: https://x.test\n")
            .unwrap();
        temp.child("apps/broken").create_dir_all().unwrap();

        let config = crate::config::load_from_path(&temp.path().join("config.yaml")).unwrap();
        let state = AppState::new(config);
        state.reload_apps(&temp.path().join("config.yaml"));

        let snap = state.dock_snapshot();
        assert_eq!(snap.apps.len(), 1);
        assert_eq!(snap.apps[0].name, "Good App");
        assert_eq!(snap.errors.len(), 1, "errors were {:?}", snap.errors);
        assert!(snap.errors[0].contains("broken"), "errors were {:?}", snap.errors);
    }
}
