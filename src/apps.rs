use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppSizeHint {
    Ledger,
    #[default]
    Folio,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AppManifest {
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub size: AppSizeHint,
    /// Local panel-focused hotkey (validated/registered in a later task).
    #[serde(default)]
    pub hotkey: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedApp {
    pub dir: PathBuf,
    pub manifest: AppManifest,
}

#[derive(Debug, Clone)]
pub struct AppLoadError {
    pub dir: PathBuf,
    pub message: String,
}

const MANIFEST_FILE: &str = "app.curt.yaml";

/// Scan `apps_dir` one level deep. Each subdirectory must contain a valid
/// `app.curt.yaml`. Invalid directories are reported, never fatal — the shell
/// loads with whatever is valid. A missing `apps_dir` means zero apps.
pub fn load_apps(apps_dir: &Path) -> (Vec<LoadedApp>, Vec<AppLoadError>) {
    let mut loaded = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(apps_dir) else {
        return (loaded, errors);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match std::fs::read_to_string(path.join(MANIFEST_FILE)) {
            Ok(raw) => match parse_manifest(&raw) {
                Ok(m) => loaded.push(LoadedApp { dir: path, manifest: m }),
                Err(e) => errors.push(AppLoadError { dir: path, message: e }),
            },
            Err(e) if e.kind() == ErrorKind::NotFound => errors.push(AppLoadError {
                dir: path,
                message: format!("missing {MANIFEST_FILE}"),
            }),
            Err(e) => errors.push(AppLoadError {
                dir: path,
                message: format!("cannot read {MANIFEST_FILE}: {e}"),
            }),
        }
    }
    loaded.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    errors.sort_by(|a, b| a.dir.cmp(&b.dir));
    (loaded, errors)
}

fn parse_manifest(raw: &str) -> Result<AppManifest, String> {
    let m: AppManifest =
        serde_yaml::from_str(raw).map_err(|e| format!("invalid {MANIFEST_FILE}: {e}"))?;
    if m.url.is_some() == m.entry.is_some() {
        return Err("exactly one of url or entry is required".to_string());
    }
    Ok(m)
}

/// Relative `apps_dir` config values resolve against the config file's directory.
pub fn resolve_apps_dir(config_dir: &Path, apps_dir: &Path) -> PathBuf {
    if apps_dir.is_absolute() {
        apps_dir.to_path_buf()
    } else {
        config_dir.join(apps_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::path::{Path, PathBuf};

    fn valid_manifest() -> &'static str {
        "name: \"Curtis Chat\"\nurl: \"https://app.curtisai.tech\"\nsize: full\n"
    }

    #[test]
    fn parses_remote_url_manifest() {
        let m: AppManifest = serde_yaml::from_str(valid_manifest()).unwrap();
        assert_eq!(m.name, "Curtis Chat");
        assert_eq!(m.url.as_deref(), Some("https://app.curtisai.tech"));
        assert_eq!(m.entry, None);
        assert_eq!(m.size, AppSizeHint::Full);
    }

    #[test]
    fn defaults_size_to_folio() {
        let m: AppManifest = serde_yaml::from_str("name: X\nurl: https://x.test\n").unwrap();
        assert_eq!(m.size, AppSizeHint::Folio);
    }

    #[test]
    fn scan_finds_valid_apps_and_reports_invalid_dirs() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("good/app.curt.yaml").write_str(valid_manifest()).unwrap();
        temp.child("broken/app.curt.yaml").write_str("name: [unclosed").unwrap();
        temp.child("empty").create_dir_all().unwrap();
        temp.child("stray-file.txt").write_str("not a dir").unwrap();

        let (loaded, errors) = load_apps(temp.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].manifest.name, "Curtis Chat");
        assert_eq!(errors.len(), 2, "broken + empty dirs are errors: {:?}", errors);
        assert!(errors.iter().any(|e| e.dir.ends_with("broken")));
        assert!(errors.iter().any(|e| e.dir.ends_with("empty") && e.message.contains("missing app.curt.yaml")));
    }

    #[test]
    fn scan_returns_empty_for_missing_dir() {
        let temp = assert_fs::TempDir::new().unwrap();
        let (loaded, errors) = load_apps(&temp.path().join("nope"));
        assert!(loaded.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn rejects_manifest_with_both_url_and_entry() {
        let raw = "name: X\nurl: https://x.test\nentry: index.html\n";
        let (loaded, errors) = load_one(raw);
        assert!(loaded.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("exactly one of url or entry"));
    }

    #[test]
    fn rejects_manifest_with_neither_url_nor_entry() {
        let (loaded, errors) = load_one("name: X\n");
        assert!(loaded.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("exactly one of url or entry"));
    }

    #[test]
    fn rejects_manifest_with_unknown_fields() {
        let raw = "name: X\nurl: https://x.test\nbogus: true\n";
        let (loaded, errors) = load_one(raw);
        assert!(loaded.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("invalid app.curt.yaml"));
    }

    fn load_one(raw: &str) -> (Vec<LoadedApp>, Vec<AppLoadError>) {
        // Helper for single-manifest validation via the same path load_apps uses.
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("app/app.curt.yaml").write_str(raw).unwrap();
        load_apps(temp.path())
    }

    #[test]
    fn relative_apps_dir_resolves_against_config_dir() {
        let resolved = resolve_apps_dir(Path::new("/home/x/.curtis/panel"), Path::new("apps"));
        assert_eq!(resolved, PathBuf::from("/home/x/.curtis/panel/apps"));
        let resolved = resolve_apps_dir(Path::new("/home/x"), Path::new("/opt/apps"));
        assert_eq!(resolved, PathBuf::from("/opt/apps"));
    }
}
