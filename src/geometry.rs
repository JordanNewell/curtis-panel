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
        // Odd cross dims truncate via integer division: 1921 / 2 == 960.
        let odd = Rect { x: 0, y: 0, width: 1921, height: 1080 };
        let r = dock_rect(odd, DockEdge::Right, DockSize::Full, None);
        assert_eq!(r.width, 960);
    }

    #[test]
    fn nonzero_origin_propagates_to_all_edges() {
        let m = Rect { x: -1920, y: 400, width: 1920, height: 1080 };
        assert_eq!(dock_rect(m, DockEdge::Left, DockSize::Ledger, None).x, -1920);
        assert_eq!(dock_rect(m, DockEdge::Right, DockSize::Ledger, None).x, -360);
        assert_eq!(dock_rect(m, DockEdge::Top, DockSize::Ledger, None).y, 400);
        assert_eq!(dock_rect(m, DockEdge::Bottom, DockSize::Ledger, None).y, 400 + 1080 - 360);
    }

    #[test]
    fn bottom_custom_sizes_height_and_keeps_full_width() {
        let custom = PersistedSize { width: 500, height: 700 };
        let r = dock_rect(MONITOR, DockEdge::Bottom, DockSize::Folio, Some(custom));
        assert_eq!(r.height, 700, "custom height wins on the cross axis");
        assert_eq!(r.width, 1920);
        assert_eq!(r.y, 1080 - 700);
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
