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
