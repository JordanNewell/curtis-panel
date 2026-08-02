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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub launch_on_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            webhook_port: default_webhook_port(),
            always_on_top: false,
            launch_on_login: false,
        }
    }
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
