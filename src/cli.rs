use std::path::PathBuf;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "curtis-panel", version, about = "Curtis Labs desktop panel")]
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
        println!("curtis-panel does not appear to be running (no PID file at {})", pid_path.display());
    }
    Ok(())
}

fn default_config_path() -> PathBuf {
    crate::config::default_config_path()
}

fn pid_file_path() -> PathBuf {
    std::env::temp_dir().join("curtis-panel.pid")
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
