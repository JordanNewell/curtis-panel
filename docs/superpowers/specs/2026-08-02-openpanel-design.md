# openpanel — v1 Design Spec

**Date:** 2026-08-02
**Status:** Approved design, ready for implementation planning
**Working name:** `openpanel`
**License:** MIT
**Author:** Jordan Newell

---

## 1. Overview

**openpanel** is a desktop control panel — a software reimagining of the Work Louder × OpenAI 2026 concept hardware. It's an always-on-top, configurable button grid that pulses with live state from your systems.

Where Stream Deck is action-first (buttons exist to fire), openpanel is **signal-first** — buttons have heartbeats. The panel breathes with your fleet, your builds, your webhooks.

### Vision

- Default install is **NEWELL-branded** (OLED black `#0A0A0A`, JetBrains Mono, green `#00FF41`, no glow)
- **Fully themeable** — every color, shape, motion is configurable per theme
- **Three action types + three signal types** cover ~95% of real use cases
- **Webhook-addressable** — any process on the machine can pulse any button via `curl -X POST localhost:7331/<button-id>`
- **Config-driven** — one YAML file defines everything; hot-reloads on save
- **Open-source MIT** from day one

### Future trajectory

v1 ships software. The **spec** (button model, signal protocol, config schema, theme system) becomes the durable asset. Hardware is a separate project consideration for v2+. The brand scales: "openpanel" as the spec/software; physical device gets its own NEWELL-flavored product name when the time comes.

---

## 2. Architecture

Tauri v2 desktop app, three isolated layers:

```
┌─────────────────────────────────────────────────┐
│  Tauri shell (Rust)                             │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │  Webview (HTML/CSS/JS, no framework)  │      │
│  │  • Renders button grid                │      │
│  │  • Handles clicks                     │      │
│  │  • Reflects signal state in UI        │      │
│  └───────────────────────────────────────┘      │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │  Core (Rust)                          │      │
│  │  • Config loader (YAML)               │      │
│  │  • Action executor (shell/open/http)  │      │
│  │  • Signal runtime                     │      │
│  │     - http-poll   - file-watch        │      │
│  │     - webhook server (:7331)          │      │
│  │  • JS expression eval (boa)           │      │
│  │  • Per-button state machine           │      │
│  │  • Theme engine                       │      │
│  └───────────────────────────────────────┘      │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │  OS integration                       │      │
│  │  • System tray  • Global hotkeys      │      │
│  │  • Always-on-top  • Auto-launch       │      │
│  └───────────────────────────────────────┘      │
└─────────────────────────────────────────────────┘
```

**Why Tauri over Electron:** ~10MB binary vs ~150MB, native webview (no Chromium bundling), Rust core gives us a real signal runtime (file watches, HTTP servers, expression eval) without a Node runtime. Critical for an always-on-top desktop app.

**Layer isolation principles:**
- **Webview** is pure presentation. Doesn't know what a button *does*, only what it *looks like* given current state.
- **Core** is the brain. Loads config, runs actions, polls signals, holds state. Testable without UI.
- **OS integration** is window/chrome behavior — hotkey to show/hide, tray menu, startup.

---

## 3. Aesthetic direction

**Default theme: NEWELL Strict.** Matches Jordan's NEWELL brand system exactly:

| Token | Value |
|---|---|
| Background | `#0A0A0A` (soft black) |
| Surface | `#0F0F0F` |
| Border | `#1F1F1F` |
| Text | `#FFFFFF` |
| Muted | `#A0A0A0` |
| Accent (primary) | `#00FF41` |
| Accent lift | `#1AFF72` |
| Error | `#FF4141` |
| Glow | `0` everywhere (NEWELL Hard NO on glow effects) |
| Typography | JetBrains Mono (Newell display face for labels is v1.1) |
| Spacing | 8px grid `[4, 8, 12, 16, 24, 32, 48, 64]` |
| Radius | circle=`9999`, square=`4`, oval=`9999` |

**Button layout** mirrors the Work Louder source render — varied button shapes (circle, square, half, oval, crosshair, lightning bolt centerpiece) arranged in a 4×5 grid with span variations.

**Theme system** allows full override. Users who want the Work Louder blue-glow look can define a `work-louder-blue` theme — every color, glow, font, motion is themeable. Brand integrity is preserved by default; customization is the product feature.

---

## 4. Button model

A button is the atomic unit. Every button is fully self-contained:

```yaml
- id: miranda-brief              # unique id; used in webhooks & state lookups
  shape: square                  # circle | square | half | oval | crosshair | custom
  icon: icons/bolt.svg           # SVG path or data URL (optional)
  label: "Miranda Brief"         # tooltip / long-press label
  slot: [2, 1]                   # grid position [col, row]
  span: [1, 1]                   # col/row span (1x1 default)

  action:                        # what happens on click
    type: shell                  # shell | open | http
    command: "miranda-brief --today"
    timeout: 30s
    confirm: false               # or string for custom message
    on_success: { toast: "Brief sent", set_state: active, ttl: 60s }
    on_error:   { toast: "Failed: {stderr}" }

  signal:                        # what makes it "alive" (optional)
    type: webhook                # http-poll | file-watch | webhook | none
    alive_when: "body.status == 'ok'"
    alive_for: 60s               # revert to idle after 60s

  display:                       # how state maps to appearance
    idle:    { color: "#FFFFFF", glow: 0 }
    active:  { color: "#00FF41", glow: 0 }
    pressed: { color: "#00FF41", glow: 0 }
    error:   { color: "#FF4141", glow: 0 }

  hotkey: "Cmd+Shift+M"          # global hotkey (optional)
  theme: default                 # which theme entry to use
```

### Principles

1. **Self-contained.** Each button owns its action + signal + display. No global state coupling. Easy to test, easy to share (export = one YAML block).
2. **Signal is optional.** A button can be inert (just opens a URL) or alive (polls a metric, watches a file, receives webhooks). Both first-class.
3. **Display is declarative.** State→appearance mapping via theme. No imperative UI code.
4. **Layout = grid with span.** Handles Work Louder-style varied button sizes without custom positioning code.
5. **Webhook-addressable.** Every button with `id: foo` accepts POSTs to `localhost:7331/foo`. Any process on the machine can pulse it without polling.

---

## 5. Action types

Three types in v1. All share `timeout`, `confirm`, `prompt`, `on_success`, `on_error`, and `{env.X}` / `{prompt.X}` / `{state.X}` template substitution.

### `shell` — run command as current user
```yaml
action:
  type: shell
  command: "ssh userve1 'systemctl restart hermes-miranda'"
  timeout: 30s
  confirm: "Restart Miranda on userve1?"
  on_success: { toast: "Miranda restarted", set_state: active, ttl: 60s }
  on_error:   { toast: "Failed: {stderr}" }
```

### `open` — URL / file / app via default handler
```yaml
action:
  type: open
  target: "https://jordannewell.com"
  # also: "file:///E:/vaults/anything.xyz/COLLAB.md"
  # also: "obsidian://open?vault=anything"
```

### `http` — request with response handling
```yaml
action:
  type: http
  method: POST
  url: "https://api.linear.app/graphql"
  headers: { Authorization: "Bearer {env.LINEAR_TOKEN}" }
  body: '{ "query": "mutation {{ createTask(...) }}" }'
  expect: [200]
  timeout: 10s
```

### Cross-cutting fields

| Field | Behavior |
|---|---|
| `timeout` | Kill after duration. Default `30s`. |
| `confirm` | Gate destructive ops. `true` (generic) or custom string. |
| `prompt` | Capture input before running, substitute as `{prompt.<key>}`. Supports `options` for select. |
| `on_success` | Toast, set state with TTL, log. |
| `on_error` | Toast with `{stderr}` / `{status}` / `{error}` substitution. |
| Template subst | `{env.X}` / `{prompt.X}` / `{state.X}` throughout all string fields. |

### Deferred

- `script` (Python sidecar with button context) — v1.1
- `keys` (keyboard macro / hotkey sequence) — v1.1
- `sequence` (composite multi-step) — v1.1
- `menu` (radial sub-menus) — v1.2
- `agent` (NL → OpenClaw/Hermes) — v2

### Security stance

v1 runs everything as your user, no sandbox. Documented clearly. The `confirm` field is the only pre-execution gate. Rationale: you're writing your own config, on your own machine. Power-user tool. Sandboxed execution comes when less-technical users start sharing configs.

---

## 6. Signal layer

The differentiator. Stream Deck and Raycast are action-first. openpanel is **signal-first** — buttons have heartbeats.

### Three sources → one pipeline

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  http-poll   │   │  file-watch  │   │   webhook    │
│ (we ask)     │   │ (FS tells us)│   │ (they push)  │
└──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       └──────────────────┼──────────────────┘
                          ▼
              ┌───────────────────────┐
              │  alive_when (JS expr) │   ← boa engine (pure Rust)
              └───────────┬───────────┘
                          ▼
              ┌───────────────────────┐
              │  Button state machine │
              │  idle / active / error│
              └───────────┬───────────┘
                          ▼
              ┌───────────────────────┐
              │  Display update       │
              │  (color, glow, value) │
              └───────────────────────┘
```

### `http-poll` — periodic pull
```yaml
signal:
  type: http-poll
  source: "http://userve1:9100/miranda/status"
  interval: 30s
  timeout: 5s
  headers: { Authorization: "Bearer {env.FLEET_TOKEN}" }
  alive_when: "last_run_ago < 300"
  display_value: "'briefed ' + humanize(last_brief_ago)"
```

### `file-watch` — filesystem push (no polling)
```yaml
signal:
  type: file-watch
  source: "~/.hermes/logs/curator/"
  event: create                    # create | modify | delete | any
  glob: "*.json"
  alive_when: "mtime > Date.now() - 300000"
  display_value: "filename"
```

### `webhook` — receive pushes from anything
```yaml
signal:
  type: webhook
  secret: "{env.PANEL_HOOK_SECRET}"   # HMAC-SHA256, optional
  alive_when: "body.status == 'ok'"
  alive_for: 60s                       # revert to idle after 60s
```

Webhook server listens on `localhost:7331`. **Any process** can pulse any button with `curl -X POST localhost:7331/<button-id>`. Agent crons, build scripts, CI runs, scheduled tasks — all become panel drivers without writing config.

### Cross-cutting signal concepts

| Concept | Behavior |
|---|---|
| **State machine** | `idle` ↔ `active` ↔ `error`. TTL on `active` (via `alive_for`) reverts to `idle`. |
| **`alive_when`** | JS expression evaluated against signal data. Pure-Rust `boa` engine, no subprocess. |
| **`display_value`** | Optional JS expression; result string shows under/inside button (e.g., "2m ago", "P95: 234ms"). |
| **Error state** | Network fail / parse fail / eval fail → `error` (`#FF4141`). Configurable `max_failures` to suppress flapping. |
| **Debounce** | `debounce: 5s` — no more than one state change per N seconds. Prevents jittery signals. |
| **Multiple signals per button** | v1: one signal per button. v1.1: aggregate (`any_active` / `all_active`). |

### Performance budget

- `http-poll`: bounded concurrency (Tokio + reqwest). Min interval 5s enforced. Default 60s.
- `file-watch`: `notify` crate, OS-native (inotify / ReadDirectoryChanges / kqueue). No polling.
- `webhook`: single Axum server, all buttons multiplexed via URL path. ~1MB memory total.

---

## 7. Config format & file layout

### Why YAML (with guardrails)

YAML wins for nested button definitions. Mitigations for known gotchas (apostrophes in single-quoted strings — a real bug class):
1. Default examples use **double-quoted strings**
2. `openpanel check` CLI lints config before runtime, catches unescaped apostrophes with line numbers
3. JSON Schema for editor autocomplete + validation
4. Use `yaml-rust2` (better error messages than legacy parsers)

### File layout

```
~/.openpanel/
├── config.yaml              # main entry, includes panels/themes
├── panels/
│   ├── fleet.yaml
│   ├── build-desk.yaml
│   └── web-ops.yaml
├── themes/
│   ├── newell-strict.yaml   # default
│   └── work-louder-blue.yaml
├── icons/                   # SVG icons
├── scripts/                 # bundled scripts (v1.1)
└── state/                   # runtime state, last-known-good values
```

### `config.yaml` — top-level
```yaml
includes:
  - panels/*.yaml
  - themes/*.yaml

settings:
  webhook_port: 7331
  hotkey_show_hide: "Cmd+Shift+P"
  always_on_top: true
  launch_on_login: true
  start_minimized: false

active_panel: fleet           # which panel shows on launch
```

### `panels/fleet.yaml` — one panel
```yaml
panel:
  id: fleet
  name: "Fleet Command"
  theme: newell-strict
  grid:
    columns: 4
    rows: 5
    spacing: 8                # NEWELL 8px grid

  buttons:
    - id: miranda-brief
      slot: [2, 1]
      shape: square
      icon: icons/bolt.svg
      label: "Miranda Brief"
      action:
        type: shell
        command: "miranda-brief --today"
        on_success: { toast: "Brief sent", set_state: active, ttl: 60s }
      signal:
        type: webhook
        alive_when: "body.status == 'ok'"
        alive_for: 60s
```

### `themes/newell-strict.yaml` — default
```yaml
theme:
  id: newell-strict
  colors:
    bg: "#0A0A0A"
    border: "#1F1F1F"
    surface: "#0F0F0F"
    text: "#FFFFFF"
    muted: "#A0A0A0"
    accent: "#00FF41"
    accent_lift: "#1AFF72"
    error: "#FF4141"
  glow:                        # NEWELL: zero by default
    idle: 0
    active: 0
    pressed: 0
    error: 0
  typography:
    font_family: "JetBrains Mono"
    label_size: 11
    value_size: 10
  radius: { circle: 9999, square: 4, oval: 9999 }
  motion:
    press_duration: 100ms
    state_change_duration: 200ms
```

### Hot reload + validation

- File watcher on `~/.openpanel/` (reuses `notify` crate from signal layer)
- On change → parse → validate → **atomic state swap**
- **Bad config never breaks the running panel** — show error toast, keep old config
- `openpanel check` CLI: lint before runtime, validates against schema, reports line numbers
- `openpanel reload` CLI: force reload

### Multi-panel support

- One panel visible at a time
- Switch via hotkey (`Cmd+Shift+]` cycles), system tray menu, or edge swipe
- v1: simple cycle. v1.1: panel picker overlay.

### Examples shipped (not auto-installed)

- `examples/panels/fleet.yaml` — fleet command inspired (generic, no Jordan-specific defaults)
- `examples/panels/build-desk.yaml` — deploy/test/PR
- `examples/panels/web-ops.yaml` — publish/check
- `examples/themes/newell-strict.yaml` — canonical default
- `examples/themes/work-louder-blue.yaml` — homage variant

---

## 8. v1 scope (ships)

```
✓ Tauri v2 desktop app (Windows + macOS + Linux)
✓ YAML config (panels, themes, settings)
✓ 3 action types:  shell | open | http
✓ 3 signal types:  http-poll | file-watch | webhook
✓ Theme system (newell-strict default + examples)
✓ 6 button shapes: circle | square | half | oval | crosshair | custom
✓ Grid layout with slot/span
✓ Cross-cutting fields: timeout, confirm, prompt, on_success/on_error
✓ Hot reload with atomic swap + bad-config protection
✓ System tray, global hotkey, always-on-top, launch-on-login
✓ Multi-panel (cycle via hotkey)
✓ Webhook receiver at localhost:7331
✓ CLI: openpanel check | reload | version
✓ Examples: 3 sample panels + 2 themes
✓ JSON Schema for editor autocomplete
```

## 9. Deferred

| Version | Items |
|---|---|
| **v1.1** | `script` action (Python sidecar) · `keys` action (hotkey macros) · `sequence` action · multi-signal aggregation · Newell display face for labels |
| **v1.2** | `menu` action (radial sub-menus) · panel picker overlay · plugin architecture |
| **v2** | `agent` action (NL → OpenClaw/Hermes) · web widget mode · mobile |
| **Separate project** | Physical hardware device |

## 10. Non-goals

- Mobile app (v2+ at earliest)
- Web-only widget mode (desktop is the focus; embeddable web widget is v2)
- Cloud sync (config lives locally; users sync via git if desired)
- Built-in marketplace (community shares configs via git)
- Sandboxed script execution in v1 (power-user tool first; sandboxing when less-technical users arrive)

---

## 11. Tech stack

| Concern | Choice | Rationale |
|---|---|---|
| Shell | **Tauri v2** | Small binary, native webview, Rust core |
| Frontend | **Vanilla HTML/CSS/JS** | Panels are 10-30 buttons; framework is overhead |
| HTTP server (webhooks) | **Axum** | De-facto Rust standard |
| HTTP client | **reqwest** | De-facto Rust standard |
| File watcher | **notify** | OS-native, efficient |
| YAML | **yaml-rust2** | Better error messages |
| JS expressions | **boa** | Pure-Rust JS engine, no subprocess |
| Schema validation | **jsonschema** + **serde** | Industry standard |
| Webhook signatures | **hmac** + **sha2** | Standard crypto crates |
| CLI | **clap** | De-facto Rust standard |

## 12. Project structure

```
openpanel/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config/          # load, validate, hot reload
│   ├── action/          # shell, open, http executors
│   ├── signal/          # http_poll, file_watch, webhook
│   ├── state/           # button state machine
│   ├── theme/           # theme engine
│   ├── expr/            # boa JS expression wrapper
│   ├── window/          # Tauri window mgmt
│   ├── tray.rs          # system tray
│   └── cli.rs           # CLI subcommands
├── frontend/            # webview (HTML/CSS/JS, no build step)
│   ├── index.html
│   ├── styles/
│   └── components/
├── examples/            # sample panels + themes
├── schemas/             # config.schema.json
└── docs/                # single-page docs site
```

## 13. Ship criteria

- Binary size **< 15MB** per platform
- Cold start **< 500ms**
- Memory idle **< 50MB**
- 100 buttons with active signals: **< 5% CPU**
- Webhook latency **< 50ms**
- Config hot reload **< 200ms**
- Fresh-install experience: download → run → see NEWELL panel working in **< 60 seconds**
- CI green on Windows + macOS + Linux from day 1

## 14. Risks & mitigations

1. **Tauri webview differences** (WebView2/WKWebView/WebKitGTK) — keep CSS conservative, test all 3 platforms in CI from day 1.
2. **Always-on-top behavior** varies by OS — use Tauri's abstraction; fall back gracefully.
3. **Global hotkeys** need per-OS handling — `global-hotkey` Rust crate.
4. **NEWELL "Newell" font** isn't installed on user machines — bundle as `.woff2` in frontend assets.
5. **YAML apostrophe gotchas** — `openpanel check` linter catches them with line numbers; examples use double-quoted strings.

## 15. Future: hardware piggyback strategy

```
v1:  Software panel ships (2-3 weeks)         ← we're here
v2:  Spec opens, community forms              ← others build web/mobile plugins
v3a: Hardware once spec is proven             ← 12-18 months out
v3b: Or license to existing HW maker          ← skip supply chain entirely
```

The **brand** is the durable thing across all three stages. `openpanel` as the spec & software; physical device gets its own NEWELL-flavored product name when the time comes. The NEWELL "no glow" rule probably shouldn't apply to a physical device — different medium, different rules. For NOW (v1 software): NEWELL Strict as default, with full themeability.

## 16. Open questions

None. All design decisions resolved through brainstorming session 2026-08-02.

---

## Next step

Implementation plan via `superpowers:writing-plans`.
