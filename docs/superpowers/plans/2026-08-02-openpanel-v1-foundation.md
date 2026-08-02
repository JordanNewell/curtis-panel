# openpanel v1 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working vertical slice — Tauri app loads a single-button panel from `~/.openpanel/config.yaml`, renders it NEWELL-styled, clicking it runs a shell command.

**Architecture:** Tauri v2 shell + Rust core (config loader, state, action executor) + vanilla JS frontend. Shell action only — `open`/`http` actions, signals, full theme system, multi-panel come in subsequent plans.

**Tech Stack:** Tauri v2, Rust (serde, serde_yaml, tokio, notify, anyhow, thiserror, clap), HTML/CSS/JS (no build step).

**Scope note:** Plan 1 of 4 for openpanel v1. Produces v0.1.0-alpha. Subsequent plans expand to full v1.

---

## File Structure

```
openpanel/
├── Cargo.toml                      # Rust deps + Tauri build config
├── tauri.conf.json                 # Tauri window/bundle config
├── build.rs                        # Tauri build script
├── src/
│   ├── main.rs                     # Entry — dispatches CLI vs GUI
│   ├── lib.rs                      # Re-exports for tests
│   ├── cli.rs                      # clap CLI subcommands
│   ├── config/
│   │   ├── mod.rs                  # Public API
│   │   ├── schema.rs               # Serde structs (Config, Panel, Button, etc.)
│   │   ├── loader.rs               # YAML → Config
│   │   ├── validator.rs            # Schema + apostrophe lint
│   │   └── hot_reload.rs           # File watcher + atomic swap
│   ├── action/
│   │   ├── mod.rs                  # Action trait
│   │   └── shell.rs                # Shell executor (pure functions)
│   ├── state.rs                    # Button state machine
│   └── commands.rs                 # #[tauri::command] handlers
├── frontend/
│   ├── index.html
│   ├── styles/
│   │   └── main.css                # NEWELL tokens as CSS variables
│   └── components/
│       ├── panel.js                # Renders grid from get_panel response
│       └── button.js               # Renders one button
├── examples/
│   └── config.yaml                 # Minimal single-button example
├── schemas/
│   └── config.schema.json          # JSON Schema for editor autocomplete
└── .github/
    └── workflows/
        └── ci.yml                  # Build on Win + macOS + Linux
```

---

## Task 1: Bootstrap Tauri project skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `build.rs`
- Create: `tauri.conf.json`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `frontend/index.html`

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "openpanel"
version = "0.1.0-alpha"
edition = "2021"
description = "Signal-first desktop control panel"
license = "MIT"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
tokio = { version = "1", features = ["full"] }
notify = "6"
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
assert_fs = "1"
pretty_assertions = "1"
```

- [ ] **Step 2: Write `build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: Write `tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "openpanel",
  "version": "0.1.0-alpha",
  "identifier": "com.newell.openpanel",
  "build": {
    "frontendDist": "frontend"
  },
  "app": {
    "windows": [
      {
        "title": "openpanel",
        "width": 480,
        "height": 640,
        "alwaysOnTop": true,
        "decorations": false,
        "resizable": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": []
  }
}
```

- [ ] **Step 4: Write `src/main.rs` (placeholder)**

```rust
fn main() {
    println!("openpanel bootstrap");
}
```

- [ ] **Step 5: Write `src/lib.rs`**

```rust
// Library crate root — re-exports for integration tests.
```

- [ ] **Step 6: Write `frontend/index.html` (placeholder)**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>openpanel</title>
</head>
<body>
  <div id="panel">bootstrap</div>
</body>
</html>
```

- [ ] **Step 7: Verify it builds**

Run: `cargo build`
Expected: builds successfully (may take a few minutes for first dependency compile)

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml build.rs tauri.conf.json src/main.rs src/lib.rs frontend/index.html
git commit -m "Bootstrap Tauri project skeleton"
```

---

## Task 2: CI workflow (build on 3 platforms)

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write CI workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Linux deps
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - run: cargo build --verbose
      - run: cargo test --verbose
```

- [ ] **Step 2: Verify YAML is well-formed**

Run: `python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Expected: no output (parse OK)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "Add CI workflow for Windows, macOS, Linux"
```

---

## Task 3: Config schema structs

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/schema.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing test for schema deserialization**

Create `src/config/schema.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ButtonShape {
    Circle,
    Square,
    Half,
    Oval,
    Crosshair,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Shell {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default = "default_timeout")]
        timeout: String,
    },
}

fn default_timeout() -> String {
    "30s".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Button {
    pub id: String,
    pub shape: ButtonShape,
    #[serde(default)]
    pub label: Option<String>,
    pub slot: [u32; 2],
    #[serde(default = "default_span")]
    pub span: [u32; 2],
    pub action: Action,
}

fn default_span() -> [u32; 2] {
    [1, 1]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Panel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub theme: Option<String>,
    pub grid: Grid,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Grid {
    pub columns: u32,
    pub rows: u32,
    #[serde(default = "default_spacing")]
    pub spacing: u32,
}

fn default_spacing() -> u32 {
    8
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Settings {
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub launch_on_login: bool,
}

fn default_webhook_port() -> u16 {
    7331
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub panel: Panel,
    #[serde(default)]
    pub settings: Settings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_button_config() {
        let yaml = r#"
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
      action:
        type: shell
        command: echo
        args: ["hello"]
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.panel.id, "test");
        assert_eq!(config.panel.buttons.len(), 1);
        assert_eq!(config.panel.buttons[0].id, "btn1");
        assert_eq!(config.panel.buttons[0].shape, ButtonShape::Square);
        match &config.panel.buttons[0].action {
            Action::Shell { command, args, .. } => {
                assert_eq!(command, "echo");
                assert_eq!(args, &vec!["hello".to_string()]);
            }
        }
    }

    #[test]
    fn applies_defaults_for_omitted_fields() {
        let yaml = r#"
panel:
  id: test
  name: Test
  grid:
    columns: 4
    rows: 5
  buttons:
    - id: btn1
      shape: circle
      slot: [2, 2]
      action:
        type: shell
        command: ls
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.panel.buttons[0].span, [1, 1]);
        assert_eq!(config.panel.grid.spacing, 8);
        assert_eq!(config.settings.webhook_port, 7331);
    }
}
```

- [ ] **Step 2: Create `src/config/mod.rs`**

```rust
pub mod schema;

pub use schema::*;
```

- [ ] **Step 3: Update `src/lib.rs`**

```rust
pub mod config;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test config::schema::tests`
Expected: 2 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/config/ src/lib.rs
git commit -m "Add config schema structs with serde"
```

---

## Task 4: JSON Schema file

**Files:**
- Create: `schemas/config.schema.json`

- [ ] **Step 1: Write JSON Schema**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "openpanel config",
  "type": "object",
  "required": ["panel"],
  "properties": {
    "panel": {
      "type": "object",
      "required": ["id", "name", "grid", "buttons"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": "string" },
        "theme": { "type": "string" },
        "grid": {
          "type": "object",
          "required": ["columns", "rows"],
          "properties": {
            "columns": { "type": "integer", "minimum": 1 },
            "rows": { "type": "integer", "minimum": 1 },
            "spacing": { "type": "integer", "default": 8 }
          }
        },
        "buttons": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["id", "shape", "slot", "action"],
            "properties": {
              "id": { "type": "string", "pattern": "^[a-z0-9-]+$" },
              "shape": {
                "type": "string",
                "enum": ["circle", "square", "half", "oval", "crosshair", "custom"]
              },
              "label": { "type": "string" },
              "slot": {
                "type": "array",
                "items": { "type": "integer" },
                "minItems": 2,
                "maxItems": 2
              },
              "span": {
                "type": "array",
                "items": { "type": "integer" },
                "minItems": 2,
                "maxItems": 2,
                "default": [1, 1]
              },
              "action": {
                "type": "object",
                "required": ["type", "command"],
                "properties": {
                  "type": { "type": "string", "enum": ["shell"] },
                  "command": { "type": "string" },
                  "args": { "type": "array", "items": { "type": "string" } },
                  "timeout": { "type": "string", "default": "30s" }
                }
              }
            }
          }
        }
      }
    },
    "settings": {
      "type": "object",
      "properties": {
        "webhook_port": { "type": "integer", "default": 7331 },
        "always_on_top": { "type": "boolean", "default": false },
        "launch_on_login": { "type": "boolean", "default": false }
      }
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add schemas/config.schema.json
git commit -m "Add JSON Schema for editor autocomplete"
```

---

## Task 5: YAML loader

**Files:**
- Create: `src/config/loader.rs`
- Modify: `src/config/mod.rs`

- [ ] **Step 1: Write failing test**

Append to `src/config/loader.rs`:

```rust
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::loader::tests`
Expected: FAIL (loader.rs doesn't exist yet — but wait, we wrote it inline. Actually the file doesn't exist so `mod loader` fails first. Let's just write the module declaration first.)

- [ ] **Step 3: Update `src/config/mod.rs`**

```rust
pub mod loader;
pub mod schema;

pub use loader::*;
pub use schema::*;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test config::loader::tests`
Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/config/loader.rs src/config/mod.rs
git commit -m "Add YAML config loader"
```

---

## Task 6: Validator with apostrophe lint

**Files:**
- Create: `src/config/validator.rs`
- Modify: `src/config/mod.rs`

- [ ] **Step 1: Write failing test**

Create `src/config/validator.rs`:

```rust
use std::path::Path;
use anyhow::{Context, Result};
use crate::config::Config;

#[derive(Debug, PartialEq)]
pub struct ValidationWarning {
    pub line: usize,
    pub message: String,
}

pub fn validate(config: &Config) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();

    for button in &config.panel.buttons {
        if button.slot[0] > config.panel.grid.columns {
            warnings.push(ValidationWarning {
                line: 0,
                message: format!(
                    "button '{}' slot column {} exceeds grid columns {}",
                    button.id, button.slot[0], config.panel.grid.columns
                ),
            });
        }
        if button.slot[1] > config.panel.grid.rows {
            warnings.push(ValidationWarning {
                line: 0,
                message: format!(
                    "button '{}' slot row {} exceeds grid rows {}",
                    button.id, button.slot[1], config.panel.grid.rows
                ),
            });
        }
    }

    warnings
}

pub fn lint_raw_yaml(raw: &str) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line_no = i + 1;
        if has_unescaped_apostrophe_in_single_quote(line) {
            warnings.push(ValidationWarning {
                line: line_no,
                message: "possible unescaped apostrophe in single-quoted string — use '' to escape, or switch to double quotes".to_string(),
            });
        }
    }
    warnings
}

fn has_unescaped_apostrophe_in_single_quote(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('\'') {
        return false;
    }
    if let Some(idx) = trimmed.find('\'') {
        if idx > 0 {
            let before = &trimmed[..idx];
            if before.ends_with('=') || before.ends_with(':') {
                let after = &trimmed[idx + 1..];
                if let Some(end) = after.find('\'') {
                    let inner = &after[..end];
                    return inner.contains('\'');
                }
            }
        }
    }
    false
}

pub fn lint_file(path: &Path) -> Result<Vec<ValidationWarning>> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(lint_raw_yaml(&contents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_unescaped_apostrophe_in_single_quote() {
        let raw = r#"
panel:
  name: 'Miranda's panel'
"#;
        let warnings = lint_raw_yaml(raw);
        assert!(!warnings.is_empty());
        assert_eq!(warnings[0].line, 3);
    }

    #[test]
    fn does_not_flag_double_quoted_strings() {
        let raw = r#"
panel:
  name: "Miranda's panel"
"#;
        let warnings = lint_raw_yaml(raw);
        assert!(warnings.is_empty());
    }

    #[test]
    fn does_not_flag_escaped_apostrophe() {
        let raw = r#"
panel:
  name: 'Miranda''s panel'
"#;
        let warnings = lint_raw_yaml(raw);
        assert!(warnings.is_empty());
    }

    #[test]
    fn flags_button_slot_out_of_bounds() {
        let config: Config = serde_yaml::from_str(r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons:
    - { id: btn1, shape: square, slot: [10, 1], action: { type: shell, command: echo } }
"#).unwrap();
        let warnings = validate(&config);
        assert!(warnings.iter().any(|w| w.message.contains("exceeds grid columns")));
    }
}
```

- [ ] **Step 2: Update `src/config/mod.rs`**

```rust
pub mod loader;
pub mod schema;
pub mod validator;

pub use loader::*;
pub use schema::*;
pub use validator::*;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test config::validator::tests`
Expected: 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add src/config/validator.rs src/config/mod.rs
git commit -m "Add config validator with apostrophe lint"
```

---

## Task 7: CLI subcommand `openpanel check`

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing test for CLI dispatch**

Create `src/cli.rs`:

```rust
use std::path::PathBuf;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "openpanel", version, about = "Signal-first desktop control panel")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<SubCmd>,
}

#[derive(Subcommand, Debug)]
pub enum SubCmd {
    /// Validate config file
    Check { path: Option<PathBuf> },
    /// Reload running panel (sends SIGHUP)
    Reload,
}

pub fn run() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Some(SubCmd::Check { path }) => run_check(path),
        Some(SubCmd::Reload) => run_reload(),
        None => crate::run_app(),
    }
}

fn run_check(path: Option<PathBuf>) -> Result<()> {
    let path = path.unwrap_or_else(default_config_path);
    println!("Checking {}...", path.display());

    let raw = std::fs::read_to_string(&path)?;
    let lint_warnings = crate::config::lint_raw_yaml(&raw);

    let config = crate::config::load_from_str(&raw)?;
    let validate_warnings = crate::config::validate(&config);

    let total = lint_warnings.len() + validate_warnings.len();
    if total == 0 {
        println!("✓ Config valid");
        return Ok(());
    }

    println!("✗ {} warning(s):", total);
    for w in &lint_warnings {
        println!("  line {}: {}", w.line, w.message);
    }
    for w in &validate_warnings {
        println!("  (semantic) {}", w.message);
    }
    std::process::exit(1);
}

fn run_reload() -> Result<()> {
    let pid_path = pid_file_path();
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        let pid: i32 = pid_str.trim().parse()?;
        #[cfg(unix)]
        unsafe {
            libc::kill(pid, libc::SIGHUP);
        }
        #[cfg(not(unix))]
        {
            println!("Reload not supported on this platform — touch the config file to hot-reload.");
        }
        println!("Reload signal sent to PID {}", pid);
    } else {
        println!("openpanel does not appear to be running (no PID file at {})", pid_path.display());
    }
    Ok(())
}

fn default_config_path() -> PathBuf {
    crate::config::default_config_path()
}

fn pid_file_path() -> PathBuf {
    std::env::temp_dir().join("openpanel.pid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn check_command_returns_ok_for_valid_config() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("valid.yaml");
        file.write_str(r#"
panel:
  id: test
  name: Test
  grid: { columns: 4, rows: 5 }
  buttons:
    - { id: btn1, shape: square, slot: [1, 1], action: { type: shell, command: echo } }
"#).unwrap();
        let raw = std::fs::read_to_string(file.path()).unwrap();
        let _config = crate::config::load_from_str(&raw).unwrap();
        let warnings = crate::config::lint_raw_yaml(&raw);
        assert!(warnings.is_empty(), "expected no warnings");
    }
}
```

- [ ] **Step 2: Update `src/main.rs`**

```rust
mod cli;
mod config;

fn main() -> anyhow::Result<()> {
    cli::run()
}

pub fn run_app() -> anyhow::Result<()> {
    println!("openpanel GUI not yet implemented — use `openpanel check` to validate config");
    Ok(())
}
```

- [ ] **Step 3: Add `libc` dep for Unix signal handling**

Modify `Cargo.toml` under `[dependencies]`:

```toml
libc = { version = "0.2", optional = true }

[features]
default = []
unix-signals = ["libc"]
```

And under `[target.'cfg(unix)'.dependencies]`:

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

(Remove the optional `libc` from `[dependencies]` to avoid duplication.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test cli::tests`
Expected: 1 test passes

- [ ] **Step 5: Verify CLI works end-to-end**

Run: `cargo run -- check examples/nonexistent.yaml 2>&1 || true`
Expected: error message about missing file

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs Cargo.toml
git commit -m "Add openpanel check + reload CLI subcommands"
```

---

## Task 8: Button state struct

**Files:**
- Create: `src/state.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing test**

Create `src/state.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ButtonState {
    Idle,
    Active,
    Pressed,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ButtonStatus {
    pub id: String,
    pub state: ButtonState,
    pub display_value: Option<String>,
    pub last_updated: u64,
}

#[derive(Debug, Default)]
pub struct StateStore {
    buttons: Arc<RwLock<HashMap<String, ButtonStatus>>>,
}

impl StateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_button(&self, id: &str) {
        let mut map = self.buttons.write();
        map.insert(
            id.to_string(),
            ButtonStatus {
                id: id.to_string(),
                state: ButtonState::Idle,
                display_value: None,
                last_updated: now_ms(),
            },
        );
    }

    pub fn set_state(&self, id: &str, state: ButtonState) {
        let mut map = self.buttons.write();
        if let Some(status) = map.get_mut(id) {
            status.state = state;
            status.last_updated = now_ms();
        }
    }

    pub fn set_display_value(&self, id: &str, value: Option<String>) {
        let mut map = self.buttons.write();
        if let Some(status) = map.get_mut(id) {
            status.display_value = value;
            status.last_updated = now_ms();
        }
    }

    pub fn get_all(&self) -> Vec<ButtonStatus> {
        let map = self.buttons.read();
        let mut v: Vec<_> = map.values().cloned().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_button_creates_idle_entry() {
        let store = StateStore::new();
        store.init_button("btn1");
        let all = store.get_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "btn1");
        assert_eq!(all[0].state, ButtonState::Idle);
    }

    #[test]
    fn set_state_updates_existing_button() {
        let store = StateStore::new();
        store.init_button("btn1");
        store.set_state("btn1", ButtonState::Active);
        let all = store.get_all();
        assert_eq!(all[0].state, ButtonState::Active);
    }

    #[test]
    fn set_state_silent_on_unknown_button() {
        let store = StateStore::new();
        store.set_state("nonexistent", ButtonState::Error);
        assert!(store.get_all().is_empty());
    }
}
```

- [ ] **Step 2: Add `parking_lot` dependency**

Modify `Cargo.toml`:

```toml
parking_lot = "0.12"
```

- [ ] **Step 3: Update `src/main.rs`**

```rust
mod cli;
mod config;
mod state;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test state::tests`
Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/state.rs Cargo.toml src/main.rs
git commit -m "Add button state store"
```

---

## Task 9: Shell action executor

**Files:**
- Create: `src/action/mod.rs`
- Create: `src/action/shell.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing test**

Create `src/action/shell.rs`:

```rust
use std::process::Command;
use std::time::Duration;
use anyhow::{anyhow, Result};
use crate::config::Action;

#[derive(Debug)]
pub struct ActionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

pub fn execute(action: &Action) -> Result<ActionResult> {
    match action {
        Action::Shell { command, args, timeout } => {
            execute_shell(command, args, parse_duration(timeout)?)
        }
    }
}

fn execute_shell(command: &str, args: &[String], timeout: Duration) -> Result<ActionResult> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let output = cmd.output().map_err(|e| anyhow!("failed to spawn '{}': {}", command, e))?;

    // Note: std::process::Command doesn't natively support timeout in stable Rust.
    // For Plan 1 we accept that timeout is documented but not enforced.
    // Plan 3 adds tokio::process::Command with proper timeout enforcement.

    Ok(ActionResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}

fn parse_duration(s: &str) -> Result<Duration> {
    if let Some(rest) = s.strip_suffix("ms") {
        let n: u64 = rest.parse().map_err(|_| anyhow!("invalid ms duration: {}", s))?;
        return Ok(Duration::from_millis(n));
    }
    if let Some(rest) = s.strip_suffix('s') {
        let n: u64 = rest.parse().map_err(|_| anyhow!("invalid seconds duration: {}", s))?;
        return Ok(Duration::from_secs(n));
    }
    let n: u64 = s.parse().map_err(|_| anyhow!("invalid duration: {}", s))?;
    Ok(Duration::from_secs(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_echo_command() {
        let action = Action::Shell {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            timeout: "5s".to_string(),
        };
        let result = execute(&action).unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[test]
    #[cfg(unix)]
    fn captures_nonzero_exit_unix() {
        let action = Action::Shell {
            command: "false".to_string(),
            args: vec![],
            timeout: "5s".to_string(),
        };
        let result = execute(&action).unwrap();
        assert!(!result.success);
    }

    #[test]
    #[cfg(windows)]
    fn captures_nonzero_exit_windows() {
        let action = Action::Shell {
            command: "cmd".to_string(),
            args: vec!["/C".to_string(), "exit 1".to_string()],
            timeout: "5s".to_string(),
        };
        let result = execute(&action).unwrap();
        assert!(!result.success);
    }

    #[test]
    fn returns_error_for_missing_command() {
        let action = Action::Shell {
            command: "/nonexistent/binary".to_string(),
            args: vec![],
            timeout: "5s".to_string(),
        };
        assert!(execute(&action).is_err());
    }

    #[test]
    fn parses_seconds_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("2").unwrap(), Duration::from_secs(2));
    }
}
```

- [ ] **Step 2: Create `src/action/mod.rs`**

```rust
pub mod shell;

pub use shell::{execute, ActionResult};
```

- [ ] **Step 3: Update `src/main.rs`**

```rust
mod action;
mod cli;
mod config;
mod state;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test action::shell::tests`
Expected: 4 tests pass on Unix; 4 tests pass on Windows (with platform-specific nonzero-exit test)

- [ ] **Step 5: Commit**

```bash
git add src/action/ src/main.rs
git commit -m "Add shell action executor"
```

---

## Task 10: Tauri commands (get_panel, trigger_action)

**Files:**
- Create: `src/commands.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write command handlers (logic extracted for testability)**

Create `src/commands.rs`:

```rust
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
```

- [ ] **Step 2: Update `src/main.rs` to wire up Tauri**

```rust
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
        anyhow::bail!("no config found at {}. Run `openpanel check <path>` to validate a config first.", config_path.display());
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
fn trigger_action(button_id: String, state: tauri::State<'_, AppState>) -> Result<commands::TriggerResult, String> {
    state.trigger(&button_id).map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test commands::tests`
Expected: 3 tests pass

- [ ] **Step 4: Verify the binary still builds (Tauri context may need icons)**

Run: `cargo build`
Expected: builds. If Tauri complains about missing icons, create empty placeholder icons or remove `bundle.active = true` in `tauri.conf.json` temporarily.

- [ ] **Step 5: Commit**

```bash
git add src/commands.rs src/main.rs
git commit -m "Add Tauri command handlers (get_panel, trigger_action)"
```

---

## Task 11: NEWELL base CSS

**Files:**
- Create: `frontend/styles/main.css`
- Modify: `frontend/index.html`

- [ ] **Step 1: Write CSS with NEWELL tokens**

Create `frontend/styles/main.css`:

```css
:root {
  /* NEWELL canonical tokens */
  --bg: #0A0A0A;
  --surface: #0F0F0F;
  --border: #1F1F1F;
  --text: #FFFFFF;
  --muted: #A0A0A0;
  --accent: #00FF41;
  --accent-lift: #1AFF72;
  --error: #FF4141;

  /* Glow defaults — NEWELL Strict = 0 */
  --glow-idle: 0;
  --glow-active: 0;
  --glow-pressed: 0;
  --glow-error: 0;

  /* Spacing — 8px grid */
  --sp-1: 4px;
  --sp-2: 8px;
  --sp-3: 12px;
  --sp-4: 16px;
  --sp-5: 24px;
  --sp-6: 32px;

  /* Radius */
  --radius-circle: 9999px;
  --radius-square: 4px;

  /* Typography */
  --font-mono: 'JetBrains Mono', 'Cascadia Mono', 'SF Mono', Consolas, monospace;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body {
  width: 100%;
  height: 100%;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12px;
  user-select: none;
  -webkit-user-select: none;
  overflow: hidden;
}

#panel {
  display: grid;
  width: 100%;
  height: 100%;
  padding: var(--sp-3);
  background: var(--bg);
}

.button {
  background: transparent;
  border: 1.5px solid var(--text);
  color: var(--text);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 100ms ease;
  position: relative;
  overflow: hidden;
}

.button.shape-circle { border-radius: var(--radius-circle); }
.button.shape-square { border-radius: var(--radius-square); }
.button.shape-half {
  border-radius: 100% 100% 0 0 / 100% 100% 0 0;
  align-self: end;
}
.button.shape-oval { border-radius: var(--radius-circle); }
.button.shape-crosshair::after {
  content: '+';
  font-size: 16px;
  color: var(--text);
}
.button.shape-custom { border-radius: var(--radius-square); }

.button.state-idle { border-color: var(--text); }
.button.state-active {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(0, 255, 65, 0.08);
}
.button.state-pressed {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--bg);
}
.button.state-error {
  border-color: var(--error);
  color: var(--error);
}

.button-label {
  position: absolute;
  bottom: -22px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 10px;
  color: var(--muted);
  white-space: nowrap;
  opacity: 0;
  transition: opacity 150ms ease;
  pointer-events: none;
}

.button:hover .button-label { opacity: 1; }
```

- [ ] **Step 2: Update `frontend/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>openpanel</title>
  <link rel="stylesheet" href="styles/main.css">
</head>
<body>
  <div id="panel"></div>
  <script src="components/button.js"></script>
  <script src="components/panel.js"></script>
  <script src="main.js"></script>
</body>
</html>
```

- [ ] **Step 3: Commit**

```bash
git add frontend/
git commit -m "Add NEWELL base CSS and index.html"
```

---

## Task 12: Frontend button + panel components

**Files:**
- Create: `frontend/components/button.js`
- Create: `frontend/components/panel.js`
- Create: `frontend/main.js`

- [ ] **Step 1: Write button component**

Create `frontend/components/button.js`:

```javascript
function createButtonElement(button, status) {
  const el = document.createElement('button');
  el.className = `button shape-${button.shape} state-${status.state}`;
  el.dataset.buttonId = button.id;

  const label = document.createElement('span');
  label.className = 'button-label';
  label.textContent = button.label || button.id;
  el.appendChild(label);

  if (status.display_value) {
    const value = document.createElement('span');
    value.className = 'button-value';
    value.textContent = status.display_value;
    el.appendChild(value);
  }

  return el;
}

window.createButtonElement = createButtonElement;
```

- [ ] **Step 2: Write panel component**

Create `frontend/components/panel.js`:

```javascript
function renderPanel(panel, states) {
  const container = document.getElementById('panel');
  container.innerHTML = '';
  container.style.gridTemplateColumns = `repeat(${panel.grid.columns}, 1fr)`;
  container.style.gridTemplateRows = `repeat(${panel.grid.rows}, 1fr)`;
  container.style.gap = `${panel.grid.spacing}px`;

  const stateById = new Map(states.map(s => [s.id, s]));

  for (const button of panel.buttons) {
    const status = stateById.get(button.id) || { state: 'idle', display_value: null };
    const el = createButtonElement(button, status);
    el.style.gridColumn = `${button.slot[0]} / span ${button.span[0]}`;
    el.style.gridRow = `${button.slot[1]} / span ${button.span[1]}`;
    el.addEventListener('click', () => onButtonClick(button.id));
    container.appendChild(el);
  }
}

window.renderPanel = renderPanel;
```

- [ ] **Step 3: Write main entry**

Create `frontend/main.js`:

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

- [ ] **Step 4: Commit**

```bash
git add frontend/components/ frontend/main.js
git commit -m "Add frontend panel and button components"
```

---

## Task 13: Example config

**Files:**
- Create: `examples/config.yaml`

- [ ] **Step 1: Write minimal example config**

```yaml
# openpanel example config — place at ~/.openpanel/config.yaml
panel:
  id: minimal
  name: "Minimal Example"
  grid:
    columns: 2
    rows: 2

  buttons:
    - id: say-hello
      shape: square
      slot: [1, 1]
      label: "Say Hello"
      action:
        type: shell
        command: echo
        args: ["hello, world"]

    - id: list-home
      shape: circle
      slot: [2, 1]
      label: "List Home"
      action:
        type: shell
        command: ls
        args: ["-la", "~"]

    - id: date-now
      shape: square
      slot: [1, 2]
      label: "Date"
      action:
        type: shell
        command: date

    - id: whoami
      shape: circle
      slot: [2, 2]
      label: "Whoami"
      action:
        type: shell
        command: whoami

settings:
  webhook_port: 7331
  always_on_top: true
```

- [ ] **Step 2: Verify it lints clean**

Run: `cargo run -- check examples/config.yaml`
Expected: `✓ Config valid`

- [ ] **Step 3: Commit**

```bash
git add examples/config.yaml
git commit -m "Add minimal example config"
```

---

## Task 14: End-to-end smoke test

**Files:**
- Create: `tests/smoke.rs`

- [ ] **Step 1: Write integration smoke test**

Create `tests/smoke.rs`:

```rust
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
```

- [ ] **Step 2: Update `src/lib.rs` to expose modules for integration tests**

```rust
pub mod action;
pub mod commands;
pub mod config;
pub mod state;
```

- [ ] **Step 3: Update `src/main.rs` to use the library**

```rust
use openpanel::cli;

fn main() -> anyhow::Result<()> {
    cli::run()
}
```

Wait — `cli::run()` needs to call `run_app()` which is currently in main.rs. Move it to lib.rs.

Update `src/lib.rs`:

```rust
pub mod action;
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
```

Move the Tauri commands from `src/main.rs` into `src/commands.rs`:

In `src/commands.rs` append:

```rust
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
```

And update `src/cli.rs` to use `crate::run_app()` → it already does.

And update `src/main.rs` to:

```rust
fn main() -> anyhow::Result<()> {
    openpanel::cli::run()
}
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test --test smoke`
Expected: 2 tests pass

- [ ] **Step 5: Commit**

```bash
git add tests/smoke.rs src/lib.rs src/main.rs src/commands.rs
git commit -m "Add end-to-end smoke test; restructure into lib + bin"
```

---

## Task 15: Tag v0.1.0-alpha

- [ ] **Step 1: Update Cargo.toml version (already `0.1.0-alpha`, verify)**

Run: `grep version Cargo.toml`
Expected: `version = "0.1.0-alpha"`

- [ ] **Step 2: Update README with build/run instructions**

Modify `README.md`:

```markdown
# openpanel

A signal-first desktop control panel.

**Status:** v0.1.0-alpha — Foundation. Shell action only. Signals, full themes, and remaining features come in subsequent plans.

## Build

```bash
cargo build --release
```

## Configure

```bash
mkdir -p ~/.openpanel
cp examples/config.yaml ~/.openpanel/config.yaml
```

Edit `~/.openpanel/config.yaml` to define your buttons.

## Run

```bash
cargo run --release
```

Or validate config without running GUI:

```bash
cargo run -- check ~/.openpanel/config.yaml
```

## Spec

See [`docs/superpowers/specs/2026-08-02-openpanel-design.md`](docs/superpowers/specs/2026-08-02-openpanel-design.md) for full v1 design.

## License

MIT
```

- [ ] **Step 3: Commit README update**

```bash
git add README.md
git commit -m "Update README with v0.1.0-alpha build/run instructions"
```

- [ ] **Step 4: Tag the release**

```bash
git tag -a v0.1.0-alpha -m "v0.1.0-alpha — Foundation: shell actions, basic UI"
```

- [ ] **Step 5: Verify final test suite passes**

Run: `cargo test`
Expected: all tests pass across all modules

---

## Self-review checklist (for implementer)

After completing all 15 tasks, verify:

- [ ] `cargo build --release` succeeds on all 3 platforms
- [ ] `cargo test` — all unit + integration tests pass
- [ ] `cargo run -- check examples/config.yaml` returns `✓ Config valid`
- [ ] `cargo run --release` launches Tauri window with 2x2 button grid
- [ ] Clicking "Say Hello" button prints "hello, world" to stdout
- [ ] Clicking "Date" button updates button state visibly
- [ ] Editing `~/.openpanel/config.yaml` and saving triggers hot reload (Plan 1.1 — not yet implemented; document as known gap)

## Known gaps in Plan 1 (intentional, deferred)

These items are NOT in Plan 1 — they're in subsequent plans:

- **Hot reload** — file watcher exists in deps but not wired (Plan 1.1)
- **`open` / `http` actions** — only `shell` implemented (Plan 2)
- **All 6 button shapes rendered correctly** — CSS exists, but full visual fidelity TBD (Plan 2)
- **Full theme system** — hardcoded NEWELL defaults only (Plan 2)
- **Multi-panel** — single panel only (Plan 2)
- **Signal layer** — http-poll / file-watch / webhook (Plan 3)
- **System tray, hotkeys, launch-on-login** — not implemented (Plan 4)
- **Cross-platform binary release** — CI builds but doesn't publish (Plan 4)

## Next plan

**Plan 2: open/http actions, full theme system, all button shapes, multi-panel**

Builds on Plan 1's foundation. Brings feature set up to spec sections 5 (full action types), 3 (full NEWELL aesthetic), and starts on 7 (multi-panel).
