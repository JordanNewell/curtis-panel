use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::config::Config;

pub fn load_from_path(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    load_from_str(&contents)
}

pub fn load_from_str(contents: &str) -> Result<Config> {
    serde_yaml::from_str(contents)
        .with_context(|| "failed to parse YAML")
}

pub fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".openpanel").join("config.yaml")
}

/// Resolve the configured apps_dir against the config file's location.
/// Falls back to `<config dir>/apps` when unset.
/// A parentless `config_path` resolves against the current directory.
pub fn apps_dir_for(config: &Config, config_path: &Path) -> PathBuf {
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    crate::apps::resolve_apps_dir(
        config_dir,
        config.apps_dir.as_deref().unwrap_or(Path::new("apps")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn loads_config_from_file() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("config.yaml");
        file.write_str(r#"
panel:
  id: test
  name: Test
  grid:
    columns: 4
    rows: 5
  buttons:
    - id: btn1
      shape: square
      slot: [1, 1]
      action: { type: shell, command: echo }
"#).unwrap();
        let config = load_from_path(file.path()).unwrap();
        assert_eq!(config.panel.id, "test");
    }

    #[test]
    fn returns_error_for_missing_file() {
        let result = load_from_path(Path::new("/nonexistent/config.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_for_malformed_yaml() {
        let result = load_from_str("panel: [unclosed");
        assert!(result.is_err());
    }

    #[test]
    fn apps_dir_for_falls_back_to_config_dir_apps() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("config.yaml");
        file.write_str(r#"
panel:
  id: test
  name: Test
  grid:
    columns: 1
    rows: 1
  buttons: []
"#).unwrap();
        let config = load_from_path(file.path()).unwrap();
        let apps_dir = apps_dir_for(&config, file.path());
        assert_eq!(apps_dir, temp.path().join("apps"));
    }
}
