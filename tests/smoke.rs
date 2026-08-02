use openpanel::config::{load_from_path, validate, lint_raw_yaml};
use openpanel::commands::AppState;
use openpanel::state::ButtonState;
use std::path::PathBuf;

fn examples_config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join("config.yaml")
}

#[test]
fn example_config_loads_and_validates() {
    let path = examples_config();
    assert!(path.exists(), "example config missing at {}", path.display());

    let raw = std::fs::read_to_string(&path).unwrap();
    let lint = lint_raw_yaml(&raw);
    assert!(lint.is_empty(), "lint warnings: {:?}", lint);

    let config = load_from_path(&path).unwrap();
    let warnings = validate(&config);
    assert!(warnings.is_empty(), "validate warnings: {:?}", warnings);
}

#[test]
fn example_config_first_button_runs() {
    let config = load_from_path(&examples_config()).unwrap();
    let state = AppState::new(config);
    let first_id = state.panel_snapshot().panel.buttons[0].id.clone();
    let result = state.trigger(&first_id).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert_eq!(state.panel_snapshot().states[0].state, ButtonState::Active);
}
