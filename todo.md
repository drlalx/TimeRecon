# TimeRecon (time-recon)

Rust (Edition 2024) Desktop-Anwendung zur nachträglichen Erfassung und Visualisierung der PC-Arbeitszeit und Aktivitätsspannen.
Privacy-focused: kein Hintergrund-Daemon, keine Tasten- oder Fenstertitelüberwachung.

## Status & Abgeschlossene Meilensteine

- [x] **Cross-Platform Architektur (Windows, Linux, macOS):**
  - [x] **Windows:** Native Abfrage des Windows System Event Logs (IDs 6005/6006, Kernel-Power 42/107/506/507, Power-Troubleshooter 1).
  - [x] **Linux:** Native Abfrage von `systemd-journald` (`journalctl --list-boots -o json` und standardisierte Sleep-Message-IDs).
  - [x] **macOS (iMac / MacBook):** Auswertung von `pmset -g log` und Kernel-Boottime via `sysctl kern.boottime`.
- [x] **Terminal User Interface (TUI):**
  - [x] Ratatui (0.30) & Crossterm (0.29) basierte interaktive Terminaloberfläche mit Unicode 24h Zeitstrahl (`█`, `░`, `▓`, `·`).
  - [x] Tab-Umschaltung zwischen Übersicht und Einzelsitzungen (`Tab`, `1`, `2`).
  - [x] Interaktive Steuerung (`↑`/`↓` für Tage, `r` für Refresh, `e` für Export, `q` für Beenden).
  - [x] RAII Terminal Guard für sauberen Terminal-Reset auch bei Panics/Ctrl+C.
  - [x] 100 % MIT-lizenzierte Abhängigkeiten, 23 Tests bestanden.
- [x] **CLI Schnellbefehle:**
  - [x] `--status` / `-s` für einzeilige Statusausgabe (Waybar / i3status / tmux).
  - [x] `--export [PFAD]` / `-e` für Headless-CSV-Generierung.
  - [x] Automatische Modus-Erkennung (GUI bei Display, TUI bei Headless / SSH).
- [x] **Egui UI:**
  - [x] 24-Stunden-Zeitleiste der letzten 120 Tage mit Segmentierung (Wach, Suspend, Hibernate).
  - [x] Übersicht heutiger Sitzungen, Monatsstatistiken (Gesamt, Tagesschnitt, aktive Tage).
  - [x] CSV-Export mit plattformgerechtem Dateimanager-Aufruf (`xdg-open` / `explorer.exe` / `open`).
  - [x] Resilientes Font-Loading mit Plattformschriften und Fallback auf Egui-Standardfonts.
  - [x] Slint vollständig entfernt, schlanker Einstiegspunkt in `src/main.rs`.
- [x] **Code-Qualität & Clippy No-Panic-Direktiven:**
  - [x] `panic`, `unwrap_used`, `expect_used`, `panic_in_result_fn`, `unreachable`, `todo`, `unimplemented` auf `deny`.
  - [x] Alle 23 Tests bestanden, 0 Clippy-Warnungen.
- [x] **Lizenz-Compliance:**
  - [x] Paket-Lizenz auf MIT gesetzt (`Cargo.toml`).
  - [x] `deny.toml` Richtlinie (Ausschluss von Copyleft & Unlicensed).
  - [x] Automatisierter Test in `tests/test_license_compatibility.rs` prüft alle Abhängigkeiten über `cargo metadata`.
- [x] **Build-Skripte & Release:**
  - [x] `build.bat` für Windows.
  - [x] `build.sh` für Linux und macOS.
  - [x] Nativer Linux-Release-Build erfolgreich verifiziert (`target/release/TimeRecon`).

## Referenzprojekte

### tockler
https://github.com/MayGo/tockler (unter `referenz/tockler`)

### activitywatch
https://github.com/ActivityWatch/activitywatch (unter `referenz/activitywatch`)

### PC On/Off Time
Zeigt die am PC verbrachte Arbeitszeit des letzten Monats an, ohne Installation.
