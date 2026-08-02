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

    let _ = timeout; // documented but not enforced in v1 (Plan 3 adds tokio)

    let output = cmd.output().map_err(|e| anyhow!("failed to spawn '{}': {}", command, e))?;

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
