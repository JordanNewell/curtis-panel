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
    // Heuristic: find a single-quoted scalar opened after `:` or `=`.
    // In valid YAML, an apostrophe inside a single-quoted scalar must be
    // doubled (`''`). If the closing `'` we find is followed by non-whitespace
    // (other than a comment), the string was terminated early by an unescaped
    // apostrophe.
    let trimmed = line.trim();
    let Some(q) = trimmed.find('\'') else {
        return false;
    };
    if q == 0 {
        return false;
    }
    let before = &trimmed[..q];
    if !before.ends_with('=') && !before.ends_with(':') {
        return false;
    }
    let rest = &trimmed[q + 1..];
    let mut chars = rest.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\'' {
            // Doubled apostrophe = escaped, keep scanning.
            if chars.peek().map(|(_, nc)| *nc) == Some('\'') {
                chars.next();
                continue;
            }
            // Single closing quote. If trailing text is non-whitespace and not
            // a comment, this is a premature close from an unescaped apostrophe.
            let trailing = rest[i + 1..].trim_start();
            return !trailing.is_empty() && !trailing.starts_with('#');
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
