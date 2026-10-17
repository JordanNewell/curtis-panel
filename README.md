# curtis-panel

The Curtis desktop panel — a slide-out, edge-docked control surface for the
Curtis ecosystem. First official Curtis Labs project.

A file pulled from a drawer: hit the hotkey and the panel slides out of the
screen edge — button deck, live signals, and your Curtis apps, including the
Curtis chat platform running in-panel.

**Status:** v0.1.0-alpha — dock shell, button deck, app system (marketplace
stub). Signals runtime and the app directory come in subsequent plans.

## Tech stack

- Rust backend with [Tauri 2](https://tauri.app) (windowing, IPC)
- Vanilla JavaScript frontend (no build step; `frontend/` is served directly)
- `clap` CLI, `serde_yaml` config parsing, `tokio` runtime

## Build

```bash
cargo build --release
```

## Configure

```bash
mkdir -p ~/.curtis/panel
cp examples/config.yaml ~/.curtis/panel/config.yaml
cp -r examples/apps ~/.curtis/panel/apps
```

The panel needs a valid config at `~/.curtis/panel/config.yaml` and refuses to
start without one. `dock:` sets the mode (`dock | float | pin`), edge
(`left/right/top/bottom`), hotkey (default `alt+c`), size preset
(`ledger/folio/full`), and motion theme (`drawer/slide/stamp`). Apps are folders with an `app.curt.yaml` manifest —
see [`examples/apps/curtis-chat`](examples/apps/curtis-chat/app.curt.yaml) and
the schema at [`schemas/app.schema.json`](schemas/app.schema.json).

## Apps (marketplace stub)

Drop a folder into `~/.curtis/panel/apps/`:

```
my-app/
  app.curt.yaml   # name, url or entry, optional icon, size hint
```

Restart the panel and the app appears in the apps view. A server-side app
directory is planned for v2 — the manifest format is the contract.

## Run

```bash
cargo run --release
```

The panel starts hidden — use the hotkey or the peek tab at the docked edge.

### CLI subcommands

```bash
cargo run -- check ~/.curtis/panel/config.yaml   # validate config
cargo run -- reload                              # ask a running panel to reload
```

## Tests

```bash
cargo test
```

## Size presets

`ledger` (360px), `folio` (900px), `full` (half the screen) — switch with keys
1/2/3 or by dragging the panel edge. Drag sizes are remembered per edge.

## Project structure

```
src/            Rust backend
  cli.rs        CLI entry point (check / reload)
  commands.rs   Tauri IPC handlers + AppState
  config/       Config loading, schema, lint, validation
  apps.rs       App-manifest loader (marketplace stub)
  dock.rs       Dock state machine + persistence
  geometry.rs   Dock window geometry
  shell.rs      Window plumbing (hotkey, tray, peek, show/hide)
  action/       Button action executors (shell)
  state.rs      Per-button state store
frontend/       Vanilla JS UI (dock chrome, deck, apps, tokens)
examples/       Example config + example app
schemas/        config.schema.json, app.schema.json
tests/          Smoke tests
docs/superpowers/specs/   Design spec
```

## License

BUSL-1.1 — see [LICENSE](LICENSE). Source-available; converts to MIT on
2030-10-12. Copyright (c) 2026 Jordan Newell.
