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

    let snapshot = state.panel_snapshot();
    let triggered = snapshot.states.iter()
        .find(|s| s.id == first_id)
        .expect("triggered button should have a state entry");
    assert_eq!(triggered.state, ButtonState::Active);
}

#[test]
fn example_apps_dir_loads_cleanly() {
    let config = load_from_path(&examples_config()).unwrap();
    let apps_dir = openpanel::config::apps_dir_for(&config, &examples_config());
    assert!(apps_dir.exists(), "example apps dir missing at {}", apps_dir.display());

    let (loaded, errors) = openpanel::apps::load_apps(&apps_dir);
    assert!(errors.is_empty(), "example app errors: {:?}", errors);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].manifest.name, "Curtis Chat");
    assert_eq!(
        loaded[0].manifest.url.as_deref(),
        Some("https://app.curtisai.tech")
    );
    assert!(loaded[0].manifest.entry.is_none());
    assert_eq!(loaded[0].manifest.size, openpanel::apps::AppSizeHint::Full);
}
