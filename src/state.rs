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
