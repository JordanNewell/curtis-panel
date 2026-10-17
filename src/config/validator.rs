use std::path::Path;
use anyhow::{Context, Result};
use crate::config::{Config, DockMode};

#[derive(Debug, PartialEq)]
pub struct ValidationWarning {
    pub line: usize,
    pub message: String,
}

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

    let (key, modifiers) = parts.split_last().unwrap();
    let key = *key;
    let key_ok = key.len() == 1
        || (key.starts_with('f')
            && key[1..].parse::<u8>().is_ok()
            && (1..=24).contains(&key[1..].parse::<u8>().unwrap()));
    if !key_ok {
        warnings.push(ValidationWarning {
            line: 0,
            message: format!("hotkey key '{key}' must be a single character or F1-F24"),
        });
    }

    let mut seen: Vec<&str> = Vec::new();
    let mut order_warned = false;
    let mut last_idx: Option<usize> = None;
    for m in modifiers {
        match HOTKEY_MODIFIERS.iter().position(|c| c == m) {
            None => {
                warnings.push(ValidationWarning {
                    line: 0,
                    message: format!(
                        "unknown hotkey modifier '{m}' (expected one of: {})",
                        HOTKEY_MODIFIERS.join(", ")
                    ),
                });
            }
            Some(idx) => {
                if seen.contains(m) {
                    warnings.push(ValidationWarning {
                        line: 0,
                        message: format!("duplicate hotkey modifier '{m}'"),
                    });
                }
                if !order_warned && last_idx.is_some_and(|li| idx < li) {
                    warnings.push(ValidationWarning {
                        line: 0,
                        message: format!(
                            "hotkey modifiers must be in canonical order (ctrl, alt, shift, super, cmd) — got '{hotkey}'"
                        ),
                    });
                    order_warned = true;
                }
                last_idx = Some(idx);
            }
        }
        seen.push(m);
    }

    warnings
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

    if config.dock.mode != DockMode::Pin {
        warnings.extend(validate_hotkey(&config.dock.hotkey));
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
    let before = trimmed[..q].trim_end();
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
        assert!(!validate_hotkey("alt+ctrl+c").is_empty()); // ctrl after alt violates canonical order
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
    fn validate_includes_hotkey_warnings_in_dock_and_float_modes() {
        for mode in ["dock", "float"] {
            let config: Config = serde_yaml::from_str(&format!(r#"
panel:
  id: test
  name: Test
  grid: {{ columns: 4, rows: 5 }}
  buttons: []
dock:
  mode: {mode}
  hotkey: alt+tab
"#)).unwrap();
            let warnings = validate(&config);
            assert!(warnings.iter().any(|w| w.message.contains("reserved")), "mode={mode}");
        }
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
}
