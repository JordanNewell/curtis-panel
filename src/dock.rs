use crate::config::{DockEdge, DockSize};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedSize {
    pub width: u32,
    pub height: u32,
}

/// Self-describing over IPC: `{"preset":"folio"}` or
/// `{"custom":{"width":500,"height":800}}`. Not persisted — `DockState`
/// stores presets and overrides separately.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SizeChoice {
    Preset(DockSize),
    Custom(PersistedSize),
}

/// Runtime dock state: what the shell needs to position and show/hide the
/// window. Persisted to `dock-state.json` so edge + drag sizes survive restarts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DockState {
    pub edge: DockEdge,
    pub shown: bool,
    pub size: DockSize,
    pub custom_by_edge: HashMap<DockEdge, PersistedSize>,
    /// Last window position in `mode: float` (physical px), so a popped-out
    /// panel reopens where the consumer left it.
    pub float_pos: Option<(i32, i32)>,
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            edge: DockEdge::Right,
            shown: false,
            size: DockSize::Folio,
            custom_by_edge: HashMap::new(),
            float_pos: None,
        }
    }
}

impl DockState {
    pub fn new(edge: DockEdge, size: DockSize) -> Self {
        Self {
            edge,
            shown: false,
            size,
            custom_by_edge: HashMap::new(),
            float_pos: None,
        }
    }

    pub fn set_float_pos(&mut self, x: i32, y: i32) {
        self.float_pos = Some((x, y));
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

    /// Writes atomically: a crash mid-write can never leave a truncated
    /// `dock-state.json` behind.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let tmp: PathBuf = path.with_file_name(format!(
            "{}.tmp",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, raw).context("failed to write dock state")?;
        std::fs::rename(&tmp, path).context("failed to write dock state")?;
        Ok(())
    }

    /// `Ok(None)` when no state file exists yet (first run). Corrupt state is
    /// treated as absent; defaults apply. Any other io error propagates.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err).context("failed to read dock state"),
        };
        Ok(serde_json::from_str(&raw).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DockEdge, DockSize};

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

    #[test]
    fn float_position_persists() {
        let mut s = new_state();
        assert_eq!(s.float_pos, None);
        s.set_float_pos(120, 240);
        assert_eq!(s.float_pos, Some((120, 240)));
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join("dock-state.json");
        s.save(&path).unwrap();
        let loaded = DockState::load(&path).unwrap().unwrap();
        assert_eq!(loaded.float_pos, Some((120, 240)));
    }

    #[test]
    fn corrupt_state_is_treated_as_absent() {
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join("dock-state.json");
        std::fs::write(&path, "{not json").unwrap();
        let loaded = DockState::load(&path).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn save_creates_missing_parent_dirs() {
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join("deep/nested/dir/dock-state.json");
        new_state().save(&path).unwrap();
        assert!(DockState::load(&path).unwrap().is_some());
    }

    #[test]
    fn set_size_on_new_edge_keeps_other_edges_overrides() {
        let mut s = new_state();
        s.set_custom(500, 800); // override on Right
        s.set_edge(DockEdge::Top);
        s.set_custom(300, 400); // override on Top
        s.set_size(DockSize::Ledger); // clears Top's override only
        assert!(matches!(s.effective_size(), SizeChoice::Preset(DockSize::Ledger)));
        s.set_edge(DockEdge::Right);
        assert!(matches!(s.effective_size(), SizeChoice::Custom(PersistedSize { width: 500, height: 800 })));
    }
}
