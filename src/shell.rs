use crate::config::{DockEdge, DockMode};
use crate::dock::SizeChoice;
use crate::geometry::{dock_rect, Rect};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Window, WebviewWindow};

/// Peek tab dimensions — mirror tauri.conf.json's peek window (6x120).
const PEEK_LEN: i32 = 120;
const PEEK_THICK: i32 = 6;

/// Must match the frontend slide-out animation duration (Task 9 CSS).
const HIDE_DELAY_MS: u64 = 240;

/// A human's first real click always lands later than this.
const PEEK_ARM_DELAY_MS: u64 = 750;

/// Suppresses the focus toggle fired by peek's own programmatic `show()` at
/// startup; a human's first real click always lands later than this.
static PEEK_ARMED: AtomicBool = AtomicBool::new(false);

fn monitor_rect(window: &WebviewWindow) -> Rect {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    match monitor {
        Some(m) => {
            let pos = m.position();
            let size = m.size();
            Rect { x: pos.x, y: pos.y, width: size.width, height: size.height }
        }
        None => Rect { x: 0, y: 0, width: 1920, height: 1080 },
    }
}

/// Move + resize the main window to its docked rectangle. Called at startup
/// and whenever edge/size changes.
pub fn position_dock(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let monitor = monitor_rect(&window);
    let state: tauri::State<crate::commands::AppState> = app.state();
    let (dock_cfg, dock) = {
        let cfg = state.config.read();
        let dock = state.dock.read();
        (cfg.dock.clone(), dock.clone())
    };

    if dock_cfg.mode == DockMode::Pin {
        // openpanel v1 behavior: normal always-on-top window, centered.
        window.set_always_on_top(true)?;
        return Ok(());
    }

    if dock_cfg.mode == DockMode::Float {
        // Popped-out panel: a normal frameless window. No edge pinning, no
        // peek; reopens at the consumer's last dragged position.
        window.set_always_on_top(false)?;
        match dock.float_pos {
            Some((x, y)) => window.set_position(tauri::PhysicalPosition::new(x, y))?,
            None => window.center()?,
        }
        if dock.shown {
            window.show()?;
            let _ = window.set_focus();
        }
        return Ok(());
    }

    let custom = match dock.effective_size() {
        SizeChoice::Custom(c) => Some(c),
        SizeChoice::Preset(_) => None,
    };
    let rect = dock_rect(monitor, dock.edge, dock.size, custom);

    window.set_always_on_top(true)?;
    window.set_size(tauri::PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(tauri::PhysicalPosition::new(rect.x, rect.y))?;
    if dock.shown {
        window.show()?;
        let _ = window.set_focus();
    }
    Ok(())
}

/// Show the panel at its docked rectangle, or hide it after the CSS
/// slide-out. The hidden window is repositioned to the dock rect so the next
/// show is already correctly placed. The CSS slide animation runs in the
/// webview; window-level moves are instant.
pub fn set_dock_shown(app: &AppHandle, shown: bool) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let state: tauri::State<crate::commands::AppState> = app.state();
    {
        let mut dock = state.dock.write();
        dock.shown = shown;
    }
    persist_state(app);
    let _ = window.emit("dock://visibility", shown);

    if shown {
        position_dock(app)?;
        window.show()?;
        let _ = window.set_focus();
    } else {
        // Wait for the CSS slide-out, then hide + re-place at the dock rect.
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(HIDE_DELAY_MS));
            // Re-check: a show inside the delay window must not be undone.
            let still_hidden = !handle
                .state::<crate::commands::AppState>()
                .dock
                .read()
                .shown;
            if still_hidden {
                let _ = handle.get_webview_window("main").map(|w| w.hide());
                let _ = position_dock(&handle);
            }
        });
    }
    Ok(())
}

pub fn toggle_dock(app: &AppHandle) -> tauri::Result<()> {
    let state: tauri::State<crate::commands::AppState> = app.state();
    let shown = state.dock.read().shown;
    set_dock_shown(app, !shown)
}

pub fn hide_dock(app: &AppHandle) -> tauri::Result<()> {
    let state: tauri::State<crate::commands::AppState> = app.state();
    if !state.dock.read().shown {
        return Ok(());
    }
    set_dock_shown(app, false)
}

pub(crate) fn persist_state(app: &AppHandle) {
    let state: tauri::State<crate::commands::AppState> = app.state();
    let dock = state.dock.read().clone();
    if let Err(e) = dock.save(&crate::config::default_dock_state_path()) {
        tracing::warn!("failed to persist dock state: {e}");
    }
}

/// Drag-resize: record the new cross dimension for the current edge.
pub fn note_resize(window: &Window) {
    let app = window.app_handle();
    let state: tauri::State<crate::commands::AppState> = app.state();
    {
        let cfg = state.config.read();
        if cfg.dock.mode != DockMode::Dock {
            return;
        }
        // Programmatic resizes (startup positioning, hidden re-placement) also
        // fire Resized; only a visible window can be drag-resized by a human.
        if !state.dock.read().shown {
            return;
        }
    }
    if let Ok(size) = window.outer_size() {
        state.dock.write().set_custom(size.width, size.height);
        persist_state(app);
    }
}

/// Float mode: remember where the consumer dragged the panel.
pub fn note_move(window: &Window) {
    let app = window.app_handle();
    let state: tauri::State<crate::commands::AppState> = app.state();
    {
        let cfg = state.config.read();
        if cfg.dock.mode != DockMode::Float {
            return;
        }
    }
    if let Ok(pos) = window.outer_position() {
        state.dock.write().set_float_pos(pos.x, pos.y);
        persist_state(app);
    }
}

/// Place the peek tab at the docked edge, centered along it.
pub fn position_peek(app: &AppHandle) -> tauri::Result<()> {
    let Some(peek) = app.get_webview_window("peek") else {
        return Ok(());
    };
    let Some(main) = app.get_webview_window("main") else {
        return Ok(());
    };
    let monitor = monitor_rect(&main);
    let state: tauri::State<crate::commands::AppState> = app.state();
    let edge = state.dock.read().edge;
    let mid_x = monitor.x + monitor.width as i32 / 2;
    let mid_y = monitor.y + monitor.height as i32 / 2;
    let (x, y, w, h) = match edge {
        DockEdge::Right => (
            monitor.x + monitor.width as i32 - PEEK_THICK,
            mid_y - PEEK_LEN / 2,
            PEEK_THICK,
            PEEK_LEN,
        ),
        DockEdge::Left => (monitor.x, mid_y - PEEK_LEN / 2, PEEK_THICK, PEEK_LEN),
        DockEdge::Top => (mid_x - PEEK_LEN / 2, monitor.y, PEEK_LEN, PEEK_THICK),
        DockEdge::Bottom => (
            mid_x - PEEK_LEN / 2,
            monitor.y + monitor.height as i32 - PEEK_THICK,
            PEEK_LEN,
            PEEK_THICK,
        ),
    };
    peek.set_size(tauri::PhysicalSize::new(w as u32, h as u32))?;
    peek.set_position(tauri::PhysicalPosition::new(x, y))?;
    peek.show()?;
    let handle = peek.app_handle().clone();
    peek.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Focused(true))
            && PEEK_ARMED.load(Ordering::Relaxed)
        {
            let _ = toggle_dock(&handle);
        }
    });
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(PEEK_ARM_DELAY_MS));
        PEEK_ARMED.store(true, Ordering::Relaxed);
    });
    Ok(())
}
