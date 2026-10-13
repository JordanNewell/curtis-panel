# openpanel

A signal-first desktop control panel.

A software reimagining of the Work Louder × OpenAI 2026 concept hardware. Always-on-top, configurable button grid that pulses with live state from your systems.

**Status:** v0.1.0-alpha — Foundation. Shell action only. Signals, full themes, and remaining features come in subsequent plans.

## Build

```bash
cargo build --release
```

## Configure

```bash
mkdir -p ~/.openpanel
cp examples/config.yaml ~/.openpanel/config.yaml
```

Edit `~/.openpanel/config.yaml` to define your buttons.

## Run

```bash
cargo run --release
```

Or validate config without running GUI:

```bash
cargo run -- check ~/.openpanel/config.yaml
```

## Spec

See [`docs/superpowers/specs/2026-08-02-openpanel-design.md`](docs/superpowers/specs/2026-08-02-openpanel-design.md) for full v1 design.

## License

BUSL-1.1 — see [LICENSE](LICENSE). Source-available; converts to MIT on 2030-10-12. Copyright (c) 2026 Jordan Newell.
