# curtis-panel v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the openpanel foundation into curtis-panel — a slide-out, edge-docked, hotkey-summoned desktop panel (Records Office identity) with the Curtis chat web UI as its first embedded app and a local app-manifest system as the marketplace stub.

**Architecture:** Tauri v2. Pure-Rust core (dock config, app-manifest loader, dock state machine, dock geometry) stays testable headless; the Tauri shell wires global hotkey, edge docking, blur-hide, tray, and a peek-tab window; the vanilla-JS frontend renders deck + app views with CSS-driven motion themes. Chat embeds as a remote-url app view in an iframe — no auth code, no backend changes.

**Tech Stack:** Rust, Tauri 2 (+ `tray-icon` feature, `tauri-plugin-global-shortcut`), serde_yaml, clap, vanilla JS/CSS.

**Spec:** `docs/superpowers/specs/2026-10-17-curtis-panel-v1-design.md`

**Repo conventions:** crate name stays `openpanel` internally (spec §9 leaves the internal rename to ship time). Conventional-commit style (`feat:`, `test:`, `chore:`, `docs:`). NEVER pass `--no-verify`. The pre-commit gitleaks hook runs on every commit. Work happens on the current branch `implement-v1-foundation`. **Do not commit `README.md` as a side effect of other tasks** — it carries an unrelated uncommitted edit owned by Jordan (S670 README sweep); only Task 10 touches README.

**Size preset definitions** (used by Task 6 and Task 8): the panel spans the full docked-axis of the monitor (a column when side-docked, a row when top/bottom-docked). Presets control the *cross* dimension: `ledger` = 360px, `folio` = 900px, `full` = 50% of the monitor's cross dimension. A drag-resize overrides the preset per edge and persists.

---

### Task 1: `dock` config section + `apps_dir`

**Files:**
- Modify: `src/config/schema.rs`
- Modify: `src/config/loader.rs`
- Modify: `schemas/config.schema.json`
- Test: inline `#[cfg(test)] mod tests` in `src/config/schema.rs`

- [ ] **Step 1: Write the failing tests**

Append to the existing `mod tests` in `src/config/schema.rs`:

```rust
    #[test]
    fn dock_defaults_apply_when_section_omitted() {
        let yaml = r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons: []
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.dock.mode, DockMode::Dock);
        assert_eq!(config.dock.edge, DockEdge::Right);
        assert_eq!(config.dock.hotkey, "alt+c");
        assert!(config.dock.peek);
        assert_eq!(config.dock.size, DockSize::Folio);
        assert_eq!(config.dock.motion, DockMotion::Drawer);
        assert_eq!(config.apps_dir, None);
    }

    #[test]
    fn dock_section_parses_explicit_values() {
        let yaml = r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons: []
dock:
  mode: pin
  edge: left
  hotkey: ctrl+shift+p
  peek: false
  size: ledger
  motion: stamp
apps_dir: /tmp/my-apps
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.dock.mode, DockMode::Pin);
        assert_eq!(config.dock.edge, DockEdge::Left);
        assert_eq!(config.dock.hotkey, "ctrl+shift+p");
        assert!(!config.dock.peek);
        assert_eq!(config.dock.size, DockSize::Ledger);
        assert_eq!(config.dock.motion, DockMotion::Stamp);
        assert_eq!(config.apps_dir, Some(std::path::PathBuf::from("/tmp/my-apps")));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config::schema`
Expected: FAIL — no `DockMode`/`Dock`/`apps_dir` in scope.

- [ ] **Step 3: Implement**

In `src/config/schema.rs`, add at the top with the other imports:

```rust
use std::path::PathBuf;
```

Add after the `Grid` struct:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockMode {
    Dock,
    Pin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockSize {
    Ledger,
    Folio,
    Full,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockMotion {
    Drawer,
    Slide,
    Stamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Dock {
    pub mode: DockMode,
    pub edge: DockEdge,
    pub hotkey: String,
    pub peek: bool,
    pub size: DockSize,
    pub motion: DockMotion,
}

impl Default for Dock {
    fn default() -> Self {
        Self {
            mode: DockMode::Dock,
            edge: DockEdge::Right,
            hotkey: "alt+c".to_string(),
            peek: true,
            size: DockSize::Folio,
            motion: DockMotion::Drawer,
        }
    }
}
```

Change the `Config` struct to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub panel: Panel,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub dock: Dock,
    #[serde(default)]
    pub apps_dir: Option<PathBuf>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib config::schema`
Expected: PASS (new tests + all existing tests green).

- [ ] **Step 5: Update `schemas/config.schema.json`**

Inside the top-level `"properties"` object, alongside the existing `"panel"`, `"settings"` entries, add:

```json
    "dock": {
      "type": "object",
      "default": {},
      "properties": {
        "mode": { "type": "string", "enum": ["dock", "pin"], "default": "dock" },
        "edge": { "type": "string", "enum": ["left", "right", "top", "bottom"], "default": "right" },
        "hotkey": { "type": "string", "default": "alt+c" },
        "peek": { "type": "boolean", "default": true },
        "size": { "type": "string", "enum": ["ledger", "folio", "full"], "default": "folio" },
        "motion": { "type": "string", "enum": ["drawer", "slide", "stamp"], "default": "drawer" }
      }
    },
    "apps_dir": { "type": "string" }
```

- [ ] **Step 6: Commit**

```bash
git add src/config/schema.rs schemas/config.schema.json
git commit -m "feat: add dock config section and apps_dir"
```

---

### Task 2: Dock validation — hotkey format + reserved hotkeys

**Files:**
- Modify: `src/config/validator.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/config/validator.rs`

- [ ] **Step 1: Write the failing tests**

Append to the existing `mod tests` in `src/config/validator.rs`:

```rust
    #[test]
    fn accepts_well_formed_hotkeys() {
        assert!(validate_hotkey("alt+c").is_empty());
        assert!(validate_hotkey("ctrl+shift+p").is_empty());
        assert!(validate_hotkey("f9").is_empty());
        assert!(validate_hotkey("cmd+k").is_empty());
    }

    #[test]
    fn rejects_malformed_hotkeys() {
        assert!(!validate_hotkey("").is_empty());
        assert!(!validate_hotkey("alt+").is_empty());
        assert!(!validate_hotkey("alt+ctrl+c").is_empty()); // key must be last
        assert!(!validate_hotkey("ctrl+ctrl+k").is_empty()); // duplicate modifier
        assert!(!validate_hotkey("meta+x").is_empty()); // unknown modifier
        assert!(!validate_hotkey("alt+space+q").is_empty()); // two keys
    }

    #[test]
    fn warns_on_reserved_hotkeys() {
        let warnings = validate_hotkey("alt+tab");
        assert!(warnings.iter().any(|w| w.message.contains("reserved")));
        let warnings = validate_hotkey("ctrl+alt+del");
        assert!(warnings.iter().any(|w| w.message.contains("reserved")));
    }

    #[test]
    fn validate_includes_hotkey_warnings_in_dock_mode() {
        let config: Config = serde_yaml::from_str(r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons: []
dock:
  hotkey: alt+tab
"#).unwrap();
        let warnings = validate(&config);
        assert!(warnings.iter().any(|w| w.message.contains("reserved")));
    }

    #[test]
    fn validate_skips_hotkey_check_in_pin_mode() {
        let config: Config = serde_yaml::from_str(r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons: []
dock:
  mode: pin
  hotkey: "not a hotkey at all"
"#).unwrap();
        let warnings = validate(&config);
        assert!(warnings.iter().all(|w| !w.message.contains("hotkey")));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config::validator`
Expected: FAIL — `validate_hotkey` not found.

- [ ] **Step 3: Implement**

In `src/config/validator.rs`, add below the `use` statements:

```rust
const HOTKEY_MODIFIERS: &[&str] = &["ctrl", "alt", "shift", "super", "cmd"];

const RESERVED_HOTKEYS: &[&str] = &[
    "alt+tab",
    "ctrl+alt+del",
    "ctrl+escape",
    "alt+f4",
    "super",
    "cmd",
];

pub fn validate_hotkey(hotkey: &str) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();
    let hotkey = hotkey.trim().to_lowercase();

    if RESERVED_HOTKEYS.contains(&hotkey.as_str()) {
        warnings.push(ValidationWarning {
            line: 0,
            message: format!("hotkey '{hotkey}' is reserved by the OS — pick another"),
        });
        return warnings;
    }

    let parts: Vec<&str> = hotkey.split('+').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        warnings.push(ValidationWarning {
            line: 0,
            message: "hotkey is empty".to_string(),
        });
        return warnings;
    }

    let (keys, modifiers) = parts.split_last().unwrap();
    if keys.len() != 1 {
        warnings.push(ValidationWarning {
            line: 0,
            message: format!("hotkey '{hotkey}' must end with exactly one key"),
        });
        return warnings;
    }

    let key = keys[0];
    let key_ok = key.len() == 1 || (key.starts_with('f') && key[1..].parse::<u8>().is_ok() && (1..=24).contains(&key[1..].parse::<u8>().unwrap()));
    if !key_ok {
        warnings.push(ValidationWarning {
            line: 0,
            message: format!("hotkey key '{key}' must be a single character or F1-F24"),
        });
    }

    let mut seen = Vec::new();
    for m in modifiers {
        if !HOTKEY_MODIFIERS.contains(m) {
            warnings.push(ValidationWarning {
                line: 0,
                message: format!("unknown hotkey modifier '{m}' (expected one of: {})", HOTKEY_MODIFIERS.join(", ")),
            });
        }
        if seen.contains(m) {
            warnings.push(ValidationWarning {
                line: 0,
                message: format!("duplicate hotkey modifier '{m}'"),
            });
        }
        seen.push(m);
    }

    warnings
}
```

Then extend `validate` — add at the end of the function, before `warnings` is returned:

```rust
    if config.dock.mode == crate::config::DockMode::Dock {
        warnings.extend(
            validate_hotkey(&config.dock.hotkey)
                .into_iter()
                .map(|mut w| {
                    w.message = format!("hotkey: {}", w.message);
                    w
                }),
        );
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib config::validator`
Expected: PASS (new + existing tests).

- [ ] **Step 5: Commit**

```bash
git add src/config/validator.rs
git commit -m "feat: validate dock hotkey format and reserved combos"
```

---

### Task 3: App-manifest loader (`src/apps.rs`)

**Files:**
- Create: `src/apps.rs`
- Modify: `src/lib.rs` (add `pub mod apps;`)
- Test: inline `#[cfg(test)] mod tests` in `src/apps.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/apps.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

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
    fn relative_apps_dir_resolves_against_config_dir() {
        let resolved = resolve_apps_dir(Path::new("/home/x/.curtis/panel"), Path::new("apps"));
        assert_eq!(resolved, PathBuf::from("/home/x/.curtis/panel/apps"));
        let resolved = resolve_apps_dir(Path::new("/home/x"), Path::new("/opt/apps"));
        assert_eq!(resolved, PathBuf::from("/opt/apps"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib apps`
Expected: FAIL — `AppManifest`, `load_apps` etc. not defined.

- [ ] **Step 3: Implement**

Fill in `src/apps.rs` above the test module:

```rust
use serde::{Deserialize, Serialize};
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
            Err(_) => errors.push(AppLoadError {
                dir: path,
                message: format!("missing {MANIFEST_FILE}"),
            }),
        }
    }
    loaded.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
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
    // (the test module from Step 1)

    fn load_one(raw: &str) -> (Vec<LoadedApp>, Vec<AppLoadError>) {
        // Helper for single-manifest validation via the same path load_apps uses.
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("app/app.curt.yaml").write_str(raw).unwrap();
        load_apps(temp.path())
    }
}
```

In `src/lib.rs`, add to the module list:

```rust
pub mod apps;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib apps`
Expected: PASS.

Note: `resolve_apps_dir` test uses `/home/x` paths — on Windows Git Bash these are still valid `PathBuf`s (no filesystem access involved), so the test is platform-neutral.

- [ ] **Step 5: Commit**

```bash
git add src/apps.rs src/lib.rs
git commit -m "feat: add app-manifest loader for local app directories"
```

---

### Task 4: Example chat app + example config dock section

**Files:**
- Create: `examples/apps/curtis-chat/app.curt.yaml`
- Modify: `examples/config.yaml`
- Modify: `tests/smoke.rs`
- Modify: `src/config/loader.rs` (add `apps_dir_for` helper)

- [ ] **Step 1: Write the failing smoke test**

Append to `tests/smoke.rs`:

```rust
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
}
```

Also add the import at the top of `tests/smoke.rs`:

```rust
use openpanel::config::apps_dir_for;
```

(keep the existing `use openpanel::config::{load_from_path, validate, lint_raw_yaml};` line as-is if you prefer two use statements.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test smoke example_apps_dir`
Expected: FAIL — `apps_dir_for` not found.

- [ ] **Step 3: Implement the helper + example files**

In `src/config/loader.rs`, add below `default_config_path`:

```rust
/// Resolve the configured apps_dir against the config file's location.
/// Falls back to `<config dir>/apps` when unset.
pub fn apps_dir_for(config: &Config, config_path: &Path) -> PathBuf {
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    match &config.apps_dir {
        Some(dir) => crate::apps::resolve_apps_dir(config_dir, dir),
        None => config_dir.join("apps"),
    }
}
```

Create `examples/apps/curtis-chat/app.curt.yaml`:

```yaml
# Curtis Chat — the seed app for the panel's app system.
# Loads the hosted Curtis web UI; login happens once in the panel and the
# webview session persists (the panel performs no auth of its own).
name: "Curtis Chat"
url: "https://app.curtisai.tech"
size: full
```

In `examples/config.yaml`, append at the end (after the `settings:` block):

```yaml

dock:
  mode: dock
  edge: right
  hotkey: alt+c
  peek: true
  size: folio
  motion: drawer

apps_dir: apps # relative to this config file
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test smoke`
Expected: PASS (existing smoke tests too — the example config gained a `dock:` section and must still validate clean).

- [ ] **Step 5: Commit**

```bash
git add examples/apps examples/config.yaml src/config/loader.rs tests/smoke.rs
git commit -m "feat: example curtis-chat app and dock config in example set"
```

---

### Task 5: Dock state machine (`src/dock.rs`)

**Files:**
- Create: `src/dock.rs`
- Modify: `src/lib.rs` (add `pub mod dock;`)
- Test: inline `#[cfg(test)] mod tests` in `src/dock.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/dock.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DockEdge, DockSize};
    use assert_fs::prelude::*;

    fn new_state() -> DockState {
        DockState::new(DockEdge::Right, DockSize::Folio)
    }

    #[test]
    fn starts_hidden() {
        let s = new_state();
        assert!(!s.shown);
    }

    #[test]
    fn toggle_flips_shown() {
        let mut s = new_state();
        s.toggle();
        assert!(s.shown);
        s.toggle();
        assert!(!s.shown);
    }

    #[test]
    fn set_size_clears_custom_for_that_edge() {
        let mut s = new_state();
        s.set_custom(500, 800);
        assert!(matches!(s.effective_size(), SizeChoice::Custom(_)));
        s.set_size(DockSize::Ledger);
        assert!(matches!(s.effective_size(), SizeChoice::Preset(DockSize::Ledger)));
    }

    #[test]
    fn custom_is_persisted_per_edge() {
        let mut s = new_state();
        s.set_custom(500, 800);
        s.set_edge(DockEdge::Top);
        assert!(matches!(s.effective_size(), SizeChoice::Preset(DockSize::Folio)));
        s.set_edge(DockEdge::Right);
        assert!(matches!(s.effective_size(), SizeChoice::Custom(PersistedSize { width: 500, height: 800 })));
    }

    #[test]
    fn saves_and_loads_roundtrip() {
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join("dock-state.json");
        let mut s = new_state();
        s.toggle();
        s.set_edge(DockEdge::Left);
        s.set_custom(420, 900);
        s.save(&path).unwrap();

        let loaded = DockState::load(&path).unwrap().expect("state should exist");
        assert_eq!(loaded, s);
    }

    #[test]
    fn load_returns_none_for_missing_file() {
        let temp = assert_fs::TempDir::new().unwrap();
        let loaded = DockState::load(&temp.path().join("nope.json")).unwrap();
        assert!(loaded.is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib dock`
Expected: FAIL — `DockState` etc. not defined.

- [ ] **Step 3: Implement**

Fill in `src/dock.rs` above the test module:

```rust
use crate::config::{DockEdge, DockSize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SizeChoice {
    Preset(DockSize),
    Custom(PersistedSize),
}

/// Runtime dock state: what the shell needs to position and show/hide the
/// window. Persisted to `dock-state.json` so edge + drag sizes survive restarts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockState {
    pub edge: DockEdge,
    pub shown: bool,
    pub size: DockSize,
    pub custom_by_edge: HashMap<DockEdge, PersistedSize>,
}

impl DockState {
    pub fn new(edge: DockEdge, size: DockSize) -> Self {
        Self {
            edge,
            shown: false,
            size,
            custom_by_edge: HashMap::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.shown = !self.shown;
    }

    pub fn set_edge(&mut self, edge: DockEdge) {
        self.edge = edge;
    }

    /// Switching to a named preset discards the drag override for the edge.
    pub fn set_size(&mut self, size: DockSize) {
        self.size = size;
        self.custom_by_edge.remove(&self.edge);
    }

    pub fn set_custom(&mut self, width: u32, height: u32) {
        self.custom_by_edge.insert(
            self.edge,
            PersistedSize { width, height },
        );
    }

    pub fn effective_size(&self) -> SizeChoice {
        match self.custom_by_edge.get(&self.edge) {
            Some(custom) => SizeChoice::Custom(*custom),
            None => SizeChoice::Preset(self.size),
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(path, raw)?;
        Ok(())
    }

    /// `Ok(None)` when no state file exists yet (first run).
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_str(&raw)?))
    }
}
```

In `src/lib.rs` add:

```rust
pub mod dock;
```

Add `serde_json = "1"` to `[dependencies]` in `Cargo.toml`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib dock`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/dock.rs src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: add dock state machine with per-edge size persistence"
```

---

### Task 6: Dock geometry (pure, TDD)

**Files:**
- Create: `src/geometry.rs`
- Modify: `src/lib.rs` (add `pub mod geometry;`)
- Test: inline `#[cfg(test)] mod tests` in `src/geometry.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/geometry.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DockEdge, DockSize};
    use crate::dock::PersistedSize;

    const MONITOR: Rect = Rect { x: 0, y: 0, width: 1920, height: 1080 };

    #[test]
    fn side_dock_spans_full_height() {
        for edge in [DockEdge::Left, DockEdge::Right] {
            for size in [DockSize::Ledger, DockSize::Folio, DockSize::Full] {
                let r = dock_rect(MONITOR, edge, size, None);
                assert_eq!(r.height, 1080, "{edge:?}/{size:?}");
                assert_eq!(r.y, 0, "{edge:?}/{size:?}");
            }
        }
    }

    #[test]
    fn right_edge_pins_to_right_side() {
        let r = dock_rect(MONITOR, DockEdge::Right, DockSize::Ledger, None);
        assert_eq!(r.width, 360);
        assert_eq!(r.x, 1920 - 360);
    }

    #[test]
    fn left_edge_pins_to_left_side() {
        let r = dock_rect(MONITOR, DockEdge::Left, DockSize::Ledger, None);
        assert_eq!(r.x, 0);
        assert_eq!(r.width, 360);
    }

    #[test]
    fn top_bottom_dock_spans_full_width_and_stacks() {
        let r = dock_rect(MONITOR, DockEdge::Top, DockSize::Ledger, None);
        assert_eq!(r.width, 1920);
        assert_eq!(r.height, 360, "cross dim comes from the ledger preset");
        assert_eq!(r.y, 0);

        let r = dock_rect(MONITOR, DockEdge::Bottom, DockSize::Ledger, None);
        assert_eq!(r.y, 1080 - 360);
    }

    #[test]
    fn folio_is_900_wide_on_side_dock() {
        let r = dock_rect(MONITOR, DockEdge::Right, DockSize::Folio, None);
        assert_eq!(r.width, 900);
    }

    #[test]
    fn full_is_half_the_monitor_cross_dim() {
        let r = dock_rect(MONITOR, DockEdge::Right, DockSize::Full, None);
        assert_eq!(r.width, 960);
        let r = dock_rect(MONITOR, DockEdge::Top, DockSize::Full, None);
        assert_eq!(r.height, 540);
    }

    #[test]
    fn custom_overrides_preset() {
        let custom = PersistedSize { width: 500, height: 700 };
        let r = dock_rect(MONITOR, DockEdge::Right, DockSize::Folio, Some(custom));
        assert_eq!(r.width, 500);
        assert_eq!(r.height, 1080, "docked axis stays full regardless of custom");
    }

    #[test]
    fn panel_never_exceeds_monitor() {
        let custom = PersistedSize { width: 9999, height: 700 };
        let r = dock_rect(MONITOR, DockEdge::Right, DockSize::Folio, Some(custom));
        assert_eq!(r.width, 1920);
        assert_eq!(r.x, 0);
    }

    #[test]
    fn small_monitor_clamps_folio() {
        let small = Rect { x: 0, y: 0, width: 800, height: 600 };
        let r = dock_rect(small, DockEdge::Right, DockSize::Folio, None);
        assert_eq!(r.width, 800);
        assert_eq!(r.x, 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib geometry`
Expected: FAIL — `Rect`/`dock_rect` not defined.

- [ ] **Step 3: Implement**

Fill in `src/geometry.rs` above the test module:

```rust
use crate::config::{DockEdge, DockSize};
use crate::dock::PersistedSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

const LEDGER_DIM: u32 = 360;
const FOLIO_DIM: u32 = 900;

fn preset_cross_dim(size: DockSize, monitor_cross: u32) -> u32 {
    match size {
        DockSize::Ledger => LEDGER_DIM,
        DockSize::Folio => FOLIO_DIM,
        DockSize::Full => monitor_cross / 2,
    }
    .min(monitor_cross)
}

/// Compute the panel's window rectangle for a docked edge.
/// The panel spans the full docked axis; presets size the cross axis.
pub fn dock_rect(
    monitor: Rect,
    edge: DockEdge,
    size: DockSize,
    custom: Option<PersistedSize>,
) -> Rect {
    let horizontal = matches!(edge, DockEdge::Left | DockEdge::Right);
    let (cross, docked) = if horizontal {
        (monitor.width, monitor.height)
    } else {
        (monitor.height, monitor.width)
    };

    let cross_dim = match custom {
        // Custom only ever sizes the cross axis; docked axis stays full.
        Some(c) => (if horizontal { c.width } else { c.height }).min(cross),
        None => preset_cross_dim(size, cross),
    };

    match edge {
        DockEdge::Left => Rect { x: monitor.x, y: monitor.y, width: cross_dim, height: docked },
        DockEdge::Right => Rect {
            x: monitor.x + monitor.width as i32 - cross_dim as i32,
            y: monitor.y,
            width: cross_dim,
            height: docked,
        },
        DockEdge::Top => Rect { x: monitor.x, y: monitor.y, width: docked, height: cross_dim },
        DockEdge::Bottom => Rect {
            x: monitor.x,
            y: monitor.y + monitor.height as i32 - cross_dim as i32,
            width: docked,
            height: cross_dim,
        },
    }
}
```

In `src/lib.rs` add:

```rust
pub mod geometry;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib geometry`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/geometry.rs src/lib.rs
git commit -m "feat: add dock window geometry"
```

---

### Task 7: AppState extension + new IPC commands

**Files:**
- Modify: `src/commands.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/commands.rs`

- [ ] **Step 1: Write the failing tests**

Append to the existing `mod tests` in `src/commands.rs`:

```rust
    use crate::apps::AppManifest;
    use crate::config::{DockEdge, DockSize};
    use crate::dock::DockState;

    fn test_state_with_dock() -> AppState {
        let config = test_config();
        AppState::new(config)
    }

    #[test]
    fn dock_snapshot_lists_loaded_apps() {
        let state = test_state_with_dock();
        state.set_apps_for_test(vec![AppManifest {
            name: "Curtis Chat".into(),
            icon: None,
            url: Some("https://app.curtisai.tech".into()),
            entry: None,
            size: crate::apps::AppSizeHint::Full,
        }]);
        let snap = state.dock_snapshot();
        assert_eq!(snap.edge, DockEdge::Right);
        assert!(matches!(snap.motion, crate::config::DockMotion::Drawer));
        assert_eq!(snap.apps.len(), 1);
        assert_eq!(snap.apps[0].name, "Curtis Chat");
    }

    #[test]
    fn set_dock_size_updates_state_and_clears_custom() {
        let state = test_state_with_dock();
        state.dock.write().set_custom(500, 800);
        state.set_dock_size(DockSize::Ledger);
        let snap = state.dock_snapshot();
        assert_eq!(snap.size, DockSize::Ledger);
        assert!(snap.custom.is_none());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib commands`
Expected: FAIL — `set_apps_for_test`, `dock_snapshot`, `set_dock_size` not found.

- [ ] **Step 3: Implement**

In `src/commands.rs`, extend the imports at the top:

```rust
use crate::apps::{load_apps, AppManifest, AppSizeHint};
use crate::config::{apps_dir_for, DockSize};
use crate::dock::{DockState, SizeChoice};
```

Extend `AppState`:

```rust
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub state: StateStore,
    pub dock: parking_lot::RwLock<DockState>,
    pub apps: parking_lot::RwLock<Vec<crate::apps::LoadedApp>>,
    pub app_errors: parking_lot::RwLock<Vec<crate::apps::AppLoadError>>,
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
            dock: parking_lot::RwLock::new(dock),
            apps: parking_lot::RwLock::new(Vec::new()),
            app_errors: parking_lot::RwLock::new(Vec::new()),
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
            edge: dock.edge,
            shown: dock.shown,
            size: dock.size,
            custom: match dock.effective_size() {
                SizeChoice::Preset(_) => None,
                SizeChoice::Custom(c) => Some(c),
            },
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
        }
    }

    // test hook — not used in production paths
    #[cfg(test)]
    pub fn set_apps_for_test(&self, manifests: Vec<AppManifest>) {
        *self.apps.write() = manifests
            .into_iter()
            .map(|manifest| crate::apps::LoadedApp {
                dir: std::path::PathBuf::from("/dev/null"),
                manifest,
            })
            .collect();
    }
}
```

Add the snapshot types (next to `PanelSnapshot`):

```rust
#[derive(Debug, serde::Serialize, Clone)]
pub struct AppInfo {
    pub name: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub entry: Option<String>,
    pub size: AppSizeHint,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DockSnapshot {
    pub edge: crate::config::DockEdge,
    pub shown: bool,
    pub size: DockSize,
    pub custom: Option<crate::dock::PersistedSize>,
    pub motion: crate::config::DockMotion,
    pub peek: bool,
    pub apps: Vec<AppInfo>,
}
```

Add new Tauri commands at the bottom (before the test module):

```rust
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
```

In `src/lib.rs`, register the new commands:

```rust
.invoke_handler(tauri::generate_handler![
            commands::get_panel,
            commands::trigger_action,
            commands::get_dock,
            commands::set_dock_size
        ])
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib commands`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/commands.rs src/lib.rs
git commit -m "feat: expose dock snapshot and size commands over IPC"
```

---

### Task 8: Shell wiring — default paths, hotkey, docking, tray, peek window

This task is window plumbing; most of it can't be unit-tested. Verification is `cargo build` + the manual checklist at the end. All state transitions still go through the tested `DockState`/`dock_rect`.

**Files:**
- Modify: `Cargo.toml` (deps)
- Modify: `tauri.conf.json`
- Create: `capabilities/default.json`
- Modify: `src/config/loader.rs` (default path rebrand)
- Modify: `src/cli.rs` (pid file rebrand, help text)
- Modify: `src/lib.rs` (run_app: plugins, hotkey, tray, blur-hide, peek)
- Modify: `src/commands.rs` (dock persistence path helper if needed)

- [ ] **Step 1: Dependencies**

In `Cargo.toml`, change the tauri line and add the plugin:

```toml
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
```

Run: `cargo build`
Expected: compiles (first build after adding plugin takes a while).

- [ ] **Step 2: Capabilities**

Create `capabilities/default.json`:

```json
{
  "$schema": "../schemas/capability-schema.json",
  "identifier": "default",
  "description": "Panel main + peek windows",
  "windows": ["main", "peek"],
  "permissions": [
    "core:default",
    "core:window:allow-set-position",
    "core:window:allow-set-size",
    "core:window:allow-set-focus",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:event:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered"
  ]
}
```

If the build errors on an unknown permission identifier, drop that specific permission line, rebuild, and note it — `core:default` is the load-bearing set; the window permissions are only needed if the frontend ever positions windows directly (it does not in v1, so removing them is safe).

- [ ] **Step 3: tauri.conf.json — window becomes the dock surface**

Replace the `app.windows` array and add the plugin entry. Full file:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "curtis-panel",
  "version": "0.1.0-alpha",
  "identifier": "com.newell.openpanel",
  "build": {
    "frontendDist": "frontend"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "curtis-panel",
        "visible": false,
        "decorations": false,
        "resizable": true,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "width": 900,
        "height": 800
      },
      {
        "label": "peek",
        "title": "curtis-panel",
        "visible": false,
        "decorations": false,
        "resizable": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "transparent": true,
        "width": 6,
        "height": 120
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": false,
    "targets": "all",
    "icon": ["icons/icon.png", "icons/icon.ico"]
  }
}
```

Notes: `identifier` stays `com.newell.openpanel` — changing it changes the WebView2 data directory and would orphan the persisted profile; do that only at ship time. `visible: false` — the panel never appears unsummoned; the peek tab or hotkey brings it.

- [ ] **Step 4: Rebrand default paths**

In `src/config/loader.rs`, change `default_config_path`:

```rust
pub fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".curtis").join("panel").join("config.yaml")
}
```

Add below it:

```rust
pub fn default_dock_state_path() -> PathBuf {
    default_config_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("dock-state.json")
}
```

In `src/cli.rs`, change `pid_file_path` to:

```rust
fn pid_file_path() -> PathBuf {
    std::env::temp_dir().join("curtis-panel.pid")
}
```

and the `Cli` command metadata to `#[command(name = "curtis-panel", version, about = "Curtis Labs desktop panel")]`. Also update the two user-facing strings in `run_check`/`run_reload` that mention `openpanel` to say `curtis-panel`.

- [ ] **Step 5: run_app — the shell**

Replace the body of `run_app` in `src/lib.rs`:

```rust
pub fn run_app() -> anyhow::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
        Manager,
    };
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

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
    if let Ok(Some(saved)) = crate::dock::DockState::load(&config::default_dock_state_path()) {
        *app_state.dock.write() = saved;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts([hotkey])?
            .with_handler(|app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    if let Err(e) = crate::shell::toggle_dock(app) {
                        tracing::error!("dock toggle failed: {e}");
                    }
                }
            })
            .build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_panel,
            commands::trigger_action,
            commands::get_dock,
            commands::set_dock_size
        ])
        .setup(move |app| {
            crate::shell::position_dock(app)?;
            if peek_enabled {
                crate::shell::position_peek(app)?;
            }
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let open = MenuItem::with_id(app, "open", "Open panel", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("curtis-panel")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => { let _ = crate::shell::toggle_dock(app); }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Click-away dismissal: blur on the main window hides the dock.
            if window.label() == "main" {
                if let tauri::WindowEvent::Focused(false) = event {
                    if let Err(e) = crate::shell::hide_dock(&window.app_handle()) {
                        tracing::error!("dock hide failed: {e}");
                    }
                }
                if let tauri::WindowEvent::Resized(_) = event {
                    // Drag-resize: persist the new cross dimension per edge.
                    crate::shell::note_resize(window);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
```

- [ ] **Step 6: The shell module**

Create `src/shell.rs` — all window plumbing lives here so `lib.rs` stays readable:

```rust
use crate::config::{DockEdge, DockMode};
use crate::dock::SizeChoice;
use crate::geometry::{dock_rect, Rect};
use tauri::{App, AppHandle, Manager, WebviewWindow};

const PEEK_LEN: f64 = 120.0;
const PEEK_THICK: f64 = 6.0;

fn monitor_rect(window: &WebviewWindow) -> Rect {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    match monitor {
        Some(m) => {
            let pos = m.position();
            let size = m.size();
            Rect { x: pos.x, y: pos.y, width: size.width, height: size.height }
        }
        None => Rect { x: 0, y: 0, width: 1920, height: 1080 },
    }
}

fn cross(state: &crate::commands::AppState, monitor: Rect, horizontal: bool) -> f64 {
    let dock = state.dock.read();
    let dim = match dock.effective_size() {
        SizeChoice::Preset(size) => {
            let cross_max = if horizontal { monitor.width } else { monitor.height };
            match size {
                crate::config::DockSize::Ledger => 360.0,
                crate::config::DockSize::Folio => 900.0,
                crate::config::DockSize::Full => (cross_max / 2) as f64,
            }
        }
        SizeChoice::Custom(c) => (if horizontal { c.width } else { c.height }) as f64,
    };
    let cross_max = if horizontal { monitor.width } else { monitor.height };
    dim.min(cross_max as f64)
}

/// Move + resize the main window to its docked rectangle. Called at startup
/// and whenever edge/size changes.
pub fn position_dock(app: &App) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let monitor = monitor_rect(&window);
    let state: tauri::State<crate::commands::AppState> = app.state();
    let (dock_cfg, dock) = {
        let cfg = state.config.read();
        let dock = state.dock.read();
        (cfg.dock.clone(), dock.clone())
    };

    if dock_cfg.mode == DockMode::Pin {
        // openpanel v1 behavior: normal always-on-top window, centered.
        window.set_always_on_top(true)?;
        return Ok(());
    }

    let horizontal = matches!(dock.edge, DockEdge::Left | DockEdge::Right);
    let cross_dim = cross(&state, monitor, horizontal) as u32;
    let custom = match dock.effective_size() {
        SizeChoice::Custom(c) => Some(c),
        SizeChoice::Preset(_) => None,
    };
    let rect = dock_rect(monitor, dock.edge, dock.size, custom);
    let _ = cross_dim; // cross_dim is already folded into `rect` via custom; preset path uses size

    window.set_always_on_top(true)?;
    window.set_size(tauri::LogicalSize::new(rect.width as f64, rect.height as f64))?;
    window.set_position(tauri::PhysicalPosition::new(rect.x, rect.y))?;
    if dock.shown {
        window.show()?;
        let _ = window.set_focus();
    }
    Ok(())
}

/// Park the panel fully off-monitor on its dock edge (hidden state), or back
/// to its docked rectangle (shown state). The CSS slide animation runs in the
/// webview; window-level moves are instant.
pub fn set_dock_shown(app: &AppHandle, shown: bool) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let state: tauri::State<crate::commands::AppState> = app.state();
    {
        let mut dock = state.dock.write();
        dock.shown = shown;
    }
    persist_state(app);
    let _ = window.emit("dock://visibility", shown);

    if shown {
        position_dock(app.app_handle())?;
        window.show()?;
        let _ = window.set_focus();
    } else {
        // Wait for the CSS slide-out, then hide + park offscreen.
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(240));
            let _ = handle.get_webview_window("main").map(|w| w.hide());
            let _ = position_dock(&handle);
        });
    }
    Ok(())
}

pub fn toggle_dock(app: &AppHandle) -> tauri::Result<()> {
    let state: tauri::State<crate::commands::AppState> = app.state();
    let shown = state.dock.read().shown;
    set_dock_shown(app, !shown)
}

pub fn hide_dock(app: &AppHandle) -> tauri::Result<()> {
    let state: tauri::State<crate::commands::AppState> = app.state();
    if !state.dock.read().shown {
        return Ok(());
    }
    set_dock_shown(app, false)
}

fn persist_state(app: &AppHandle) {
    let state: tauri::State<crate::commands::AppState> = app.state();
    let dock = state.dock.read().clone();
    if let Err(e) = dock.save(&crate::config::default_dock_state_path()) {
        tracing::warn!("failed to persist dock state: {e}");
    }
}

/// Drag-resize: record the new cross dimension for the current edge.
pub fn note_resize(window: &WebviewWindow) {
    let app = window.app_handle();
    let state: tauri::State<crate::commands::AppState> = app.state();
    let cfg = state.config.read();
    if cfg.dock.mode != DockMode::Dock {
        return;
    }
    let dock_edge = state.dock.read().edge;
    drop(cfg);
    if let Ok(size) = window.outer_size() {
        let horizontal = matches!(dock_edge, DockEdge::Left | DockEdge::Right);
        state.dock.write().set_custom(size.width, size.height);
        let _ = horizontal; // cross axis picked by set_custom consumer
        persist_state(app);
    }
}

/// Place the peek tab at the docked edge, centered along it.
pub fn position_peek(app: &App) -> tauri::Result<()> {
    let Some(peek) = app.get_webview_window("peek") else {
        return Ok(());
    };
    let Some(main) = app.get_webview_window("main") else {
        return Ok(());
    };
    let monitor = monitor_rect(&main);
    let state: tauri::State<crate::commands::AppState> = app.state();
    let edge = state.dock.read().edge;
    let (x, y, w, h) = match edge {
        DockEdge::Right => (monitor.x + monitor.width as i32 - PEEK_THICK as i32,
            monitor.y + monitor.height as i32 / 2 - PEEK_LEN as i32 / 2, PEEK_THICK, PEEK_LEN),
        DockEdge::Left => (monitor.x,
            monitor.y + monitor.height as i32 / 2 - PEEK_LEN as i32 / 2, PEEK_THICK, PEEK_LEN),
        DockEdge::Top => (monitor.x + monitor.width as i32 / 2 - PEEK_LEN as i32 / 2,
            monitor.y, PEEK_LEN, PEEK_THICK),
        DockEdge::Bottom => (monitor.x + monitor.width as i32 / 2 - PEEK_LEN as i32 / 2,
            monitor.y + monitor.height as i32 - PEEK_THICK as i32, PEEK_LEN, PEEK_THICK),
    };
    peek.set_size(tauri::LogicalSize::new(w, h))?;
    peek.set_position(tauri::PhysicalPosition::new(x, y))?;
    peek.show()?;
    let _ = peek.on_window_event(move |window, event| {
        if matches!(event, tauri::WindowEvent::Focused(true)) {
            let _ = crate::shell::toggle_dock(&window.app_handle());
        }
    });
    Ok(())
}
```

In `src/lib.rs` add `pub mod shell;` and `pub mod geometry;` if not already present.

Note on `on_window_event` for `peek`: `WebviewWindow::on_window_event` registers a handler; only one handler per window is supported in Tauri v2 — since the builder-level `on_window_event` filters on label `main`, the peek's own handler here is the only one for that window. Verify at build time; if the signature differs in the installed tauri version, register peek clicks in the builder-level handler instead (add a `"peek"` branch matching `Focused(true)` → `toggle_dock`).

- [ ] **Step 7: Build + manual verify**

Run: `cargo build`
Expected: compiles clean (warnings acceptable; fix errors).

Then create a config and run it. Use the example set:

```bash
mkdir -p ~/.curtis/panel
cp examples/config.yaml ~/.curtis/panel/config.yaml
cp -r examples/apps ~/.curtis/panel/apps
cargo run
```

Manual checklist (AEON, all must pass):
1. Panel does NOT appear on launch; peek tab visible at right edge.
2. `alt+c` slides panel in from the right; `alt+c` again slides it out.
3. Click on desktop (focus loss) slides it closed.
4. Drag left edge to resize; restart app; size is retained.
5. Click peek tab → panel opens.
6. Tray icon present; "Open panel" + "Quit" work.
7. `cargo run -- check` still passes against the example config.

Do not proceed to Task 9 until 1–7 pass.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock tauri.conf.json capabilities src/lib.rs src/shell.rs src/config/loader.rs src/cli.rs
git commit -m "feat: dock shell with global hotkey, edge docking, tray, peek tab"
```

---

### Task 9: Frontend — Records Office retheme + dock chrome + app views

**Files:**
- Create: `frontend/tokens.css`
- Modify: `frontend/index.html`
- Modify: `frontend/main.js`
- Create: `frontend/apps.js`
- Create: `frontend/dock.js`
- Modify: `frontend/styles/main.css`
- Modify: `frontend/components/panel.js` (namespace-safe: keep `renderPanel`, no other change)

- [ ] **Step 1: Tokens**

Create `frontend/tokens.css` — copy the full token block from
`E:/dev/projects/curtis-console/tokens.css` (frozen B0 set) verbatim, including
the `.theme-ink` override block. Do not invent values; if that file is
unavailable, extract tokens from `curtis-ai-tech/tokens.json` instead. The
block starts `:root {` and ends after the `.theme-ink` rule.

- [ ] **Step 2: Markup**

Replace `frontend/index.html` body with:

```html
<body class="theme-paper">
  <div id="dock" class="edge-right motion-drawer">
    <header id="dock-head">
      <span id="dock-mark">◉ curtis panel</span>
      <nav id="dock-tabs">
        <button class="tab active" data-view="deck">deck</button>
        <button class="tab" data-view="apps">apps</button>
      </nav>
      <button id="theme-toggle" title="toggle ink">ink</button>
    </header>
    <main id="dock-body">
      <section id="view-deck" class="view active">
        <div id="panel"></div>
      </section>
      <section id="view-apps" class="view">
        <aside id="app-list"></aside>
        <div id="app-region">
          <iframe id="app-frame" hidden></iframe>
          <div id="app-unavailable" hidden>
            <p class="stamp">RECORD UNAVAILABLE</p>
            <p id="app-unavailable-name"></p>
            <button id="app-retry">retry</button>
          </div>
        </div>
      </section>
    </main>
  </div>
  <script src="components/button.js"></script>
  <script src="components/panel.js"></script>
  <script src="apps.js"></script>
  <script src="dock.js"></script>
  <script src="main.js"></script>
</body>
```

(`head` keeps `<link rel="stylesheet" href="styles/main.css">`; add
`<link rel="stylesheet" href="tokens.css">` **before** it.)

- [ ] **Step 3: Dock behavior script**

Create `frontend/dock.js`:

```javascript
// Dock chrome: tabs, theme, slide-in/out coordination, size presets.
(function () {
  const invoke = window.__TAURI__.core.invoke;
  const listen = window.__TAURI__.event.listen;

  // --- theme toggle (paper default, persisted) ---
  const themeBtn = document.getElementById('theme-toggle');
  function applyTheme(theme) {
    document.body.classList.toggle('theme-ink', theme === 'ink');
    document.body.classList.toggle('theme-paper', theme !== 'ink');
    themeBtn.textContent = theme === 'ink' ? 'paper' : 'ink';
    localStorage.setItem('curtis-panel-theme', theme);
  }
  applyTheme(localStorage.getItem('curtis-panel-theme') || 'paper');
  themeBtn.addEventListener('click', () =>
    applyTheme(document.body.classList.contains('theme-ink') ? 'paper' : 'ink'));

  // --- view tabs ---
  document.querySelectorAll('#dock-tabs .tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('#dock-tabs .tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById(`view-${tab.dataset.view}`).classList.add('active');
    });
  });

  // --- size presets (keyboard: 1/2/3 while panel shown) ---
  window.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    const sizes = { '1': 'ledger', '2': 'folio', '3': 'full' };
    if (sizes[e.key]) invoke('set_dock_size', { size: sizes[e.key] });
  });

  // --- slide coordination with the Rust shell ---
  const dock = document.getElementById('dock');

  async function initEdgeAndMotion() {
    const snap = await invoke('get_dock');
    dock.classList.remove('edge-right', 'edge-left', 'edge-top', 'edge-bottom');
    dock.classList.add(`edge-${snap.edge}`, `motion-${snap.motion}`);
    if (snap.shown) dock.classList.add('open');
    window.dispatchEvent(new CustomEvent('dock:snapshot', { detail: snap }));
  }

  listen('dock://visibility', (event) => {
    const shown = event.payload;
    if (shown) {
      dock.classList.add('open');
    } else {
      dock.classList.remove('open'); // Rust hides the window after 240ms
    }
  });

  initEdgeAndMotion().catch(console.error);
})();
```

- [ ] **Step 4: App views script**

Create `frontend/apps.js`:

```javascript
// App system: render the installed-app list, drive the app view (iframe),
// handle the "record unavailable" state with auto-retry.
(function () {
  const invoke = window.__TAURI__.core.invoke;
  const list = document.getElementById('app-list');
  const frame = document.getElementById('app-frame');
  const unavailable = document.getElementById('app-unavailable');
  const unavailableName = document.getElementById('app-unavailable-name');
  const retryBtn = document.getElementById('app-retry');

  let activeApp = null;
  let loadTimer = null;
  let retryDelay = 5000;

  function showUnavailable(name) {
    frame.hidden = true;
    frame.src = 'about:blank';
    unavailable.hidden = false;
    unavailableName.textContent = name;
    scheduleRetry();
  }

  function scheduleRetry() {
    clearTimeout(loadTimer);
    loadTimer = setTimeout(() => { if (activeApp) openApp(activeApp); }, retryDelay);
    retryDelay = Math.min(retryDelay * 2, 60000);
  }

  function openApp(app) {
    activeApp = app;
    unavailable.hidden = true;
    retryDelay = 5000;
    clearTimeout(loadTimer);
    frame.hidden = false;
    // Load timeout: if the iframe never fires `load` in 6s, treat as down.
    loadTimer = setTimeout(() => {
      if (!frame.contentWindow || frame.contentWindow.length === undefined) showUnavailable(app.name);
    }, 6000);
    frame.src = app.url || app.entry;
  }

  frame.addEventListener('load', () => {
    clearTimeout(loadTimer);
    unavailable.hidden = true;
  });

  retryBtn.addEventListener('click', () => { if (activeApp) openApp(activeApp); });

  async function renderApps() {
    const snap = await invoke('get_dock');
    list.innerHTML = '';
    for (const app of snap.apps) {
      const btn = document.createElement('button');
      btn.className = 'app-entry';
      btn.textContent = app.name;
      btn.addEventListener('click', () => openApp(app));
      list.appendChild(btn);
    }
    if (snap.apps.length === 0) {
      const p = document.createElement('p');
      p.className = 'app-empty';
      p.textContent = 'no apps filed. drop a folder with app.curt.yaml into ~/.curtis/panel/apps/';
      list.appendChild(p);
    }
    // Marketplace placeholder (stub — v2 wires the real directory)
    const soon = document.createElement('p');
    soon.className = 'app-soon';
    soon.textContent = 'directory — coming soon';
    list.appendChild(soon);
  }

  window.addEventListener('dock:snapshot', () => renderApps().catch(console.error));
  renderApps().catch(console.error);
})();
```

- [ ] **Step 5: main.js — drop the old bootstrap duplication**

Replace `frontend/main.js` entirely:

```javascript
async function refreshPanel() {
  const snapshot = await window.__TAURI__.core.invoke('get_panel');
  renderPanel(snapshot.panel, snapshot.states);
}

async function onButtonClick(buttonId) {
  try {
    const result = await window.__TAURI__.core.invoke('trigger_action', { buttonId });
    if (!result.success) {
      console.error('Action failed:', result.stderr);
    }
    await refreshPanel();
  } catch (e) {
    console.error('Trigger error:', e);
  }
}

window.onButtonClick = onButtonClick;

refreshPanel().catch(console.error);
```

(This is identical to the current file — kept here so this task is
self-contained; if the file is unchanged since the read, skip this step.)

- [ ] **Step 6: Styles**

Replace `frontend/styles/main.css` with:

```css
/* curtis-panel — Records Office dock chrome.
   Tokens come from tokens.css (frozen B0). No hex outside tokens.css. */
@import url('../tokens.css');

* { box-sizing: border-box; }

html, body {
  margin: 0;
  height: 100%;
  background: transparent;
  font-family: var(--font-body);
  color: var(--fg);
  overflow: hidden;
  user-select: none;
}

/* The dock fills the window; the window is the docked rectangle. */
#dock {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--canvas);
  border: 1px solid var(--edge);
  /* hidden state: translate off the docked edge; .open slides in */
  transition: transform 220ms var(--motion-curve, cubic-bezier(.2, .9, .25, 1.15));
}
.motion-slide #dock { --motion-curve: ease; transition-timing-function: ease; }
.motion-stamp #dock { --motion-curve: cubic-bezier(.1, 1.4, .3, 1); }

.edge-right #dock:not(.open) { transform: translateX(100%); }
.edge-left #dock:not(.open) { transform: translateX(-100%); }
.edge-top #dock:not(.open) { transform: translateY(-100%); }
.edge-bottom #dock:not(.open) { transform: translateY(100%); }

/* header */
#dock-head {
  display: flex;
  align-items: center;
  gap: var(--grid);
  padding: var(--grid) calc(var(--grid) * 2);
  border-bottom: 1px solid var(--edge);
  font-family: var(--font-records);
  font-size: 12px;
}
#dock-mark { color: var(--accent); letter-spacing: .06em; }
#dock-tabs { display: flex; gap: calc(var(--grid) / 2); margin-left: auto; }
.tab {
  font-family: var(--font-records);
  font-size: 12px;
  background: none;
  border: 1px solid transparent;
  color: var(--fg-muted);
  padding: 2px 8px;
  cursor: pointer;
  border-radius: var(--radius-s);
}
.tab.active { color: var(--fg); border-color: var(--edge); background: var(--surface); }
#theme-toggle {
  font-family: var(--font-records);
  font-size: 12px;
  background: none;
  border: 1px solid var(--edge);
  color: var(--fg-muted);
  cursor: pointer;
  border-radius: var(--radius-s);
  padding: 2px 8px;
}

/* body layout */
#dock-body { flex: 1; min-height: 0; display: flex; }
.view { display: none; flex: 1; min-width: 0; }
.view.active { display: flex; }

/* deck view */
#view-deck { padding: var(--grid); }
#panel {
  display: grid;
  width: 100%;
  height: 100%;
}

/* apps view */
#view-apps { flex-direction: row; }
.edge-top #view-apps, .edge-bottom #view-apps { flex-direction: column; }
#app-list {
  width: 200px;
  border-right: 1px solid var(--edge);
  padding: var(--grid);
  display: flex;
  flex-direction: column;
  gap: calc(var(--grid) / 2);
  font-family: var(--font-records);
  font-size: 12px;
}
.edge-top #app-list, .edge-bottom #app-list {
  width: auto;
  border-right: none;
  border-bottom: 1px solid var(--edge);
  flex-direction: row;
  align-items: center;
}
.app-entry {
  font-family: var(--font-records);
  font-size: 12px;
  text-align: left;
  background: var(--surface);
  border: 1px solid var(--edge);
  color: var(--fg);
  padding: 6px 10px;
  cursor: pointer;
  border-radius: var(--radius-s);
}
.app-entry:hover { border-color: var(--accent); color: var(--accent); }
.app-empty { color: var(--fg-muted); max-width: 24ch; }
.app-soon { color: var(--fg-muted); font-style: italic; margin-top: auto; }

#app-region { flex: 1; min-width: 0; position: relative; background: var(--surface); }
#app-frame { width: 100%; height: 100%; border: 0; }

/* unavailable state — the one stamp-red moment */
#app-unavailable {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--grid);
  background: var(--canvas);
}
#app-unavailable .stamp {
  font-family: var(--font-records);
  color: var(--stamp-red);
  border: 2px solid var(--stamp-red);
  padding: 4px 12px;
  transform: rotate(-4deg);
  border-radius: var(--radius-s);
}
#app-unavailable-name { color: var(--fg-muted); font-family: var(--font-records); font-size: 12px; }
#app-retry {
  font-family: var(--font-records);
  background: none;
  border: 1px solid var(--edge);
  color: var(--fg);
  cursor: pointer;
  border-radius: var(--radius-s);
  padding: 4px 12px;
}

/* button grid entries keep the existing button.js classes — restyle in place */
#panel .btn {
  background: var(--surface);
  border: 1px solid var(--edge);
  color: var(--fg);
  font-family: var(--font-records);
  cursor: pointer;
  border-radius: var(--radius-s);
}
#panel .btn:hover { border-color: var(--accent); }
#panel .btn[data-state="active"] { border-color: var(--accent); color: var(--accent); }
#panel .btn[data-state="error"] { border-color: var(--stamp-red); color: var(--stamp-red); }
#panel .btn[data-state="pressed"] { background: var(--row-alt); }

/* focus rings */
button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
```

Check `frontend/components/button.js` — if its class name is not `btn` or it
doesn't set `data-state`, adapt the `#panel .btn` block to the real
class/attribute names rather than changing button.js.

- [ ] **Step 7: Manual QC**

Run the app per Task 8 Step 7, verify:
1. Paper theme default; toggle to ink persists across restart (localStorage).
2. Deck tab renders the button grid on Records Office surfaces (no NEWELL green/black).
3. Apps tab lists "Curtis Chat"; clicking it loads `app.curtisai.tech` in the region.
4. First load of chat asks for login once in-panel; after login it persists across panel restarts.
5. With network cut (or an unreachable url in a test manifest), the "RECORD UNAVAILABLE" stamp appears; retry restores.
6. Keys 1/2/3 switch size presets while shown.
7. All four edges: slide direction is correct (`--edge-*` classes) — test by editing config edge + `cargo run -- reload` or restart.
8. 220ms slide runs with drawer overshoot; switching `motion: slide` / `stamp` changes the feel.

- [ ] **Step 8: Commit**

```bash
git add frontend
git commit -m "feat: Records Office dock chrome with app views and motion themes"
```

---

### Task 10: Smoke-test coverage + README rebrand + final gates

**Files:**
- Modify: `tests/smoke.rs`
- Modify: `README.md` (replaces Jordan's pending edit — flag at review, don't silently discard)

- [ ] **Step 1: Extend smoke tests**

Append to `tests/smoke.rs`:

```rust
#[test]
fn example_config_dock_section_is_valid() {
    let config = load_from_path(&examples_config()).unwrap();
    assert_eq!(config.dock.edge, openpanel::config::DockEdge::Right);
    assert_eq!(config.dock.size, openpanel::config::DockSize::Folio);
    assert_eq!(config.dock.motion, openpanel::config::DockMotion::Drawer);
    let warnings = validate(&config);
    assert!(warnings.is_empty(), "dock validation warnings: {:?}", warnings);
}

#[test]
fn dock_state_survives_save_load_cycle() {
    use openpanel::dock::DockState;
    let temp = assert_fs::TempDir::new().unwrap();
    let path = temp.path().join("dock-state.json");
    let config = load_from_path(&examples_config()).unwrap();
    let mut state = DockState::new(config.dock.edge, config.dock.size);
    state.toggle();
    state.save(&path).unwrap();
    let loaded = DockState::load(&path).unwrap().unwrap();
    assert!(loaded.shown);
}
```

- [ ] **Step 2: Run the full suite**

Run: `cargo test`
Expected: all green (lib + smoke).

- [ ] **Step 3: Lint gates**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: zero warnings. Fix anything clippy flags (borrow patterns in Task 8 code are the likely candidates).

Run: `cargo fmt --check` — if the repo has no rustfmt.toml and files are already unformatted, skip; if formatted, fix formatting. Match existing state.

- [ ] **Step 4: README rebrand**

Replace `README.md` (the current uncommitted edit is a doc sweep pending
Jordan's S670 ruling — this task supersedes it; call that out in the session
wrap so nothing is silently discarded). New README:

```markdown
# curtis-panel

The Curtis desktop panel — a slide-out, edge-docked control surface for the
Curtis ecosystem. First official Curtis Labs project.

A file pulled from a drawer: hit the hotkey and the panel slides out of the
screen edge — button deck, live signals, and your Curtis apps, including the
Curtis chat platform running in-panel.

**Status:** v0.1.0-alpha — dock shell, button deck, app system (marketplace
stub). Signals runtime and the app directory come in subsequent plans.

## Tech stack

- Rust backend with [Tauri 2](https://tauri.app) (windowing, IPC)
- Vanilla JavaScript frontend (no build step; `frontend/` is served directly)
- `clap` CLI, `serde_yaml` config parsing, `tokio` runtime

## Build

```bash
cargo build --release
```

## Configure

```bash
mkdir -p ~/.curtis/panel
cp examples/config.yaml ~/.curtis/panel/config.yaml
cp -r examples/apps ~/.curtis/panel/apps
```

The panel needs a valid config at `~/.curtis/panel/config.yaml` and refuses to
start without one. `dock:` sets the edge (`left/right/top/bottom`), hotkey
(default `alt+c`), size preset (`ledger/folio/full`), and motion theme
(`drawer/slide/stamp`). Apps are folders with an `app.curt.yaml` manifest —
see [`examples/apps/curtis-chat`](examples/apps/curtis-chat/app.curt.yaml) and
the schema at [`schemas/app.schema.json`](schemas/app.schema.json).

## Apps (marketplace stub)

Drop a folder into `~/.curtis/panel/apps/`:

```
my-app/
  app.curt.yaml   # name, url or entry, optional icon, size hint
```

Restart (or `curtis-panel reload`) and the app appears in the panel's apps
view. A server-side app directory is planned for v2 — the manifest format is
the contract.

## Run

```bash
cargo run --release
```

The panel starts hidden — use the hotkey or the peek tab at the docked edge.

### CLI subcommands

```bash
cargo run -- check ~/.curtis/panel/config.yaml   # validate config
cargo run -- reload                              # ask a running panel to reload
```

## Tests

```bash
cargo test
```

## Size presets

`ledger` (360px), `folio` (900px), `full` (half the screen) — switch with keys
1/2/3 or by dragging the panel edge. Drag sizes are remembered per edge.

## Project structure

```
src/            Rust backend
  cli.rs        CLI entry point (check / reload)
  commands.rs   Tauri IPC handlers + AppState
  config/       Config loading, schema, lint, validation
  apps.rs       App-manifest loader (marketplace stub)
  dock.rs       Dock state machine + persistence
  geometry.rs   Dock window geometry
  shell.rs      Window plumbing (hotkey, tray, peek, show/hide)
  action/       Button action executors (shell)
  state.rs      Per-button state store
frontend/       Vanilla JS UI (dock chrome, deck, apps, tokens)
examples/       Example config + example app
schemas/        config.schema.json, app.schema.json
tests/          Smoke tests
docs/superpowers/specs/   Design spec
```

## License

BUSL-1.1 — see [LICENSE](LICENSE). Source-available; converts to MIT on
2030-10-12. Copyright (c) 2026 Jordan Newell.
```

- [ ] **Step 5: Create `schemas/app.schema.json`** (referenced by README + spec)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "curtis-panel app manifest",
  "type": "object",
  "required": ["name"],
  "properties": {
    "name": { "type": "string" },
    "icon": { "type": "string" },
    "url": { "type": "string", "format": "uri" },
    "entry": { "type": "string" },
    "size": { "type": "string", "enum": ["ledger", "folio", "full"], "default": "folio" }
  },
  "oneOf": [
    { "required": ["url"] },
    { "required": ["entry"] }
  ]
}
```

- [ ] **Step 6: Final commit**

```bash
git add tests/smoke.rs README.md schemas/app.schema.json
git commit -m "docs: rebrand README to curtis-panel, app manifest schema, smoke coverage"
```

---

## Out of plan scope (Jordan-side, after implementation)

1. **Repo rename** `openpanel` → `curtis-panel` on Gitea (`newell/`) + GitHub mirror + local directory name. Agent cannot rename Gitea repos without ruling — Jordan's call.
2. **Push word** — commits land on `implement-v1-foundation`; push on Jordan's word only.
3. **Crate rename** — spec §9 leaves internal `openpanel` crate name to ship time.
4. **curtisai.tech/labs listing** — Curtis Labs page entry (needs the site's labs section; curtis-ai-tech dev branch territory, separate session).
5. **Registry charter** — `10_PROJECTS/curtis-panel/PROJECT.md` via `/project` (WIP cap check first: building is at cap).
