# Memory — TimeRecon

- Project: `TimeRecon` (binary/package: `TimeRecon`), formerly `active-time`.
- Important items: dirs: src, referenz, tests, doc; files: Cargo.toml, deny.toml, README.md, .gitignore
- Architecture: `src/eventlog/` provides cross-platform event extraction and aggregation:
  - `src/eventlog/mod.rs` contains platform-independent domain models (`PcSession`, `DaySummary`, `Segment`, `SegmentKind`), midnight splitting, sleep merging, and CSV export.
  - `src/eventlog/windows.rs` uses the Microsoft `windows` crate and native Event Log API (events 6005/6006, Kernel-Power 42/107/506/507, Power-Troubleshooter 1).
  - `src/eventlog/linux.rs` queries `systemd-journald` via `journalctl --list-boots -o json` and sleep message IDs (`6bbd...` / `8811...`).
  - `src/eventlog/macos.rs` queries macOS power logs (`pmset -g log`) and boot times (`sysctl kern.boottime`).
- Presentation: Dual UI with feature flags:
  - `egui-ui`: Desktop GUI (`src/egui_ui.rs`) using `eframe 0.36`.
  - `tui`: Terminal User Interface (`src/tui/`) using `ratatui 0.30` and `crossterm 0.29` with 24h Unicode timeline.
  - Default features: `["egui-ui", "tui"]`.
- CLI Modes:
  - Default: launches GUI if display server (`DISPLAY`/`WAYLAND_DISPLAY`) is available; automatically falls back to TUI in headless/SSH environments.
  - Flags: `--tui` / `-t` (force TUI), `--gui` / `-g` (force GUI), `--status` / `-s` (one-line summary for statusbars/tmux), `--export [path]` / `-e` (headless CSV export).
- Clippy & Code Quality: Enforces strict no-panic lints identical to omni_wave (`panic`, `unwrap_used`, `expect_used`, `panic_in_result_fn`, `unreachable`, `todo`, `unimplemented` all set to `deny`).
- License Policy & MIT Compatibility:
  - `TimeRecon` is licensed under MIT (`license = "MIT"` in `Cargo.toml`). Author: Alexander Drichel.
  - `deny.toml` enforces strict license compliance (copyleft and unlicensed denied).
  - `tests/test_license_compatibility.rs` verifies that all dependencies across the entire crate tree have MIT-compatible licenses via `cargo metadata`.
- Multi-Platform Desktop Integration:
  - Font loading in `src/egui_ui.rs` tries platform-specific fonts (Segoe UI on Windows, DejaVu Sans / Ubuntu on Linux, SF Pro on macOS) with graceful fallback to Egui defaults.
  - CSV export opening in `src/egui_ui.rs` uses `explorer.exe` on Windows, `xdg-open` on Linux, and `open` on macOS.
- Remotes:
  - Origin: `gitea@192.168.178.45:adrichel/TimeRecon.git`
  - GitHub: `git@github.com:drlalx/TimeRecon.git`
- SigMap / Context: Configured via `gen-context.config.json`; run `sigmap` to update signatures across `.github/`, `AGENTS.md`, and `CLAUDE.md`.
- Update: Keep this file current when important findings, conventions, or dependencies change.

