# curtis-panel — v1 Design Spec

**Date:** 2026-10-17
**Status:** Approved design, ready for implementation planning
**Working name:** `curtis-panel` (repo renamed from `openpanel`)
**License:** BUSL-1.1 (converts to MIT 2030-10-12)
**Series:** First official Curtis Labs project
**Author:** Jordan Newell

---

## 1. Overview

curtis-panel is a slide-out desktop panel for the Curtis ecosystem — a frameless
window docked to any screen edge that pops open on a global hotkey. It combines
the existing openpanel button-deck (signal-first grid) with embedded Curtis web
surfaces, starting with the Curtis chat platform. A local app-manifest system
("marketplace stub") defines the format that a future server-side directory
will serve into.

Supersedes the always-on-top presentation of the 2026-08-02 openpanel spec;
that spec's button model, config system, and action layer carry forward
unchanged. Docking becomes the default; pinning stays as an option.

### Vision

- The panel slides out like a file pulled from a drawer — Records Office
  aesthetic in both look and motion.
- The Curtis chat web UI lives inside the panel; no separate window, no
  browser launch, no second login. "Web UI transitions to desktop."
- Apps are files: drop a folder with a manifest into the apps directory and it
  appears in the deck. The manifest format is the durable marketplace asset.
- Fun through customization: edges, hotkeys, sizes, and motion themes are all
  configurable.

### Out of scope (v2+)

- Server-side app directory / real marketplace
- Plugin SDK and third-party developer docs
- Signals wiring to Curtis agents and carriers (deck keeps existing signal
  spec as-is)
- Auto-launch at login, multi-monitor edge pinning per display

---

## 2. Architecture

Tauri v2, three isolated layers (unchanged layering from openpanel spec):

```
┌─────────────────────────────────────────────────┐
│  Tauri shell (Rust)                             │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │  Webview (HTML/CSS/JS, no framework)  │      │
│  │  • Dock chrome: edge, peek-tab, sizes │      │
│  │  • Deck: button grid + signals        │      │
│  │  • App views (incl. chat webview)     │      │
│  │  • Motion themes (open/close)         │      │
│  └───────────────────────────────────────┘      │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │  Core (Rust)                          │      │
│  │  • Config loader (YAML) + schema      │      │
│  │  • App-manifest loader (app.curt.yaml)│      │
│  │  • Action executor (shell)            │      │
│  │  • Signal runtime (per 2026-08-02)    │      │
│  │  • Dock state machine (shown/hidden,  │      │
│  │    edge, size preset, animation)      │      │
│  └───────────────────────────────────────┘      │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │  OS integration                       │      │
│  │  • Global hotkey (toggle)             │      │
│  │  • Edge docking + slide hide/show     │      │
│  │  • Click-away dismissal               │      │
│  │  • System tray menu (open/quit)       │      │
│  └───────────────────────────────────────┘      │
└─────────────────────────────────────────────────┘
```

**Layer isolation principles (carried forward):**
- Webview is pure presentation — knows what things look like, not what they do.
- Core is the brain — config, actions, signals, dock state; testable headless.
- OS integration is window behavior — hotkey, docking, tray.

**Chat surface:** an app view backed by a webview region loading the hosted
Curtis web UI (`app.curtisai.tech` in production, configurable URL). Session
is shared with the browser via cookie jar — the panel performs no auth of its
own. If the panel ships a chat app manifest by default, it is the seed entry
in `examples/`, not a hardcoded surface.

---

## 3. Aesthetic direction

**Records Office** (matches `curtis-ai-tech/DESIGN.md` §5 / curtis-web):

| Token | Value |
|---|---|
| Paper background | warm off-white |
| Ink (dark theme bg) | near-black |
| Accent green | single green for live/positive state |
| Stamp red | danger/proof moments only, max one per view |
| Record face | IBM Plex Mono |
| Display face | Fraunces |
| Chrome | ruled lines and receipts, no shadows/gradients |

Dark-ink variant included; paper is default. Theme toggle persisted, same
pattern as existing frontend.

**Motion themes** (open/close animation, configurable):
- `drawer` (default) — slides with slight resistance and overshoot, like a
  drawer with weight.
- `slide` — plain linear slide.
- `stamp` — snaps open fast, settles with a tiny bounce.

Motion theming is part of the theme config (timing curves, not custom code).

**The panel reads as a physical object:** deck buttons are ledger entries;
the peek-tab is a file tab; the hidden state is "filed."

---

## 4. Dock behavior

- **Edge:** docks to `left`, `right`, `top`, or `bottom` (config; default
  `right`). Orientation of deck adapts.
- **Show:** global hotkey (config; default `alt+c`, validated against common
  OS conflicts at config lint) toggles open/closed. Window slides in from the
  docked edge using the active motion theme.
- **Hide:** hotkey again, click-away (focus loss outside panel bounds), or
  tray menu. Slides out, then window hides.
- **Peek-tab:** optional 4–6px visible tab at the docked edge when hidden
  (config `peek: true|false`; default true). Click or drag opens.
- **Sizes:** three presets, snap-to-preset + drag-to-resize, persisted per
  edge in local state:
  - `ledger` — narrow deck-only (~360px wide when side-docked)
  - `folio` — deck + app view (~900px)
  - `full` — edge-to-edge along the docked axis
- **Mode:** `mode: dock` (default) or `pin` — pin restores openpanel v1
  always-on-top behavior in a normal framed position.

---

## 5. App system (marketplace stub)

- Apps live in `~/.curtis/panel/apps/<name>/`.
- Each app: `app.curt.yaml` manifest + optional local `index.html`, or a
  remote `url` (the chat app uses a remote url).
- Manifest fields: `name`, `icon` (path), `url` **or** `entry` (local html),
  `size` hint (`ledger|folio|full`), optional `hotkey` (local, panel-focused).
- Schema at `schemas/app.schema.json` (mirrors `schemas/config.schema.json`
  pattern).
- Loader: scan directory at boot + on `reload` subcommand (and SIGHUP on
  Unix). Valid manifests → launchable app views in the deck. Invalid
  manifests → skipped, logged to stderr/log file, never crash the shell.
- **Marketplace UI:** a deck view listing installed apps with open buttons +
  a "directory coming soon" placeholder. No network calls in v1.
- `examples/` ships one example app (the Curtis chat, pointing at
  `app.curtisai.tech`) to seed the format.

---

## 6. Config (extends existing YAML)

The existing top-level `panel:` key holds the grid/buttons definition, so dock
settings get their own `dock:` section (all optional, defaults shown):

```yaml
dock:
  mode: dock            # dock | pin
  edge: right           # left | right | top | bottom
  hotkey: alt+c
  peek: true
  size: folio           # ledger | folio | full
  motion: drawer        # drawer | slide | stamp
apps_dir: ~/.curtis/panel/apps   # relative paths resolve against the config file
```

Default paths rebrand from `~/.openpanel/` to `~/.curtis/panel/` (config,
`apps/`, `dock-state.json`).

Buttons, actions, signals, themes: unchanged from the 2026-08-02 spec and
current implementation. `cargo run -- check` validates the full config
including the new section; schema updated accordingly.

---

## 7. Error handling

| Failure | Behavior |
|---|---|
| Invalid config | Refuse to start; print lint + validation output (existing behavior) |
| Invalid app manifest | Skip app, log warning, shell continues |
| App URL unreachable (chat offline) | App view renders Records Office "record unavailable" state (paper, stamp); auto-retry with backoff |
| Hotkey already claimed by OS/app | Refuse to start with named conflict in lint output |
| Focus-loss during drag-resize | Size persists as-is; no snap until next deliberate drag |

---

## 8. Testing

- `cargo test` unit coverage: config parsing/validation (new panel section),
  app-manifest loader (valid/invalid/missing fields), dock state machine
  (edge/size transitions).
- `tests/smoke.rs` extended: example config + example chat-app manifest load
  and validate cleanly.
- Manual QC on AEON: hotkey toggle on all four edges, all three motion
  themes, drag-resize + preset snap, peek-tab click, chat app loads hosted
  UI and shares session, click-away dismissal, dark/paper themes.

---

## 9. Naming and repo

- Repo renamed `openpanel` → `curtis-panel` (Gitea `newell/curtis-panel`,
  GitHub mirror follows). Whether "openpanel" survives as the internal crate
  name or is renamed too is a ship-time call, not a design constraint.
- README rebranded: Curtis Labs banner, BUSL-1.1 license (unchanged),
  install/config docs updated for dock behavior and apps directory.
- Listed on curtisai.tech/labs as the first official Curtis Labs project.
