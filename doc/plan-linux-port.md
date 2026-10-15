# Plan: Linux-Portierung für `active-time`

## 1. Ziel und Rahmenbedingungen

`active-time` soll auf Linux dieselben Kernfunktionen wie unter Windows bereitstellen:
- Rekonstruktion der PC-Aktivität der letzten 120 Tage (Uptime, Boot-zu-Shutdown-Sessions).
- Erkennung von Sleep- und Hibernate-Phasen und deren Abzug von der aktiven Arbeitszeit.
- Export als CSV und Öffnen der Export-Datei im Standard-Dateimanager.
- Beibehaltung des Prinzips: Privacy-focused, kein dauerhaft laufender Überwachungs-Daemon, schlank und modular.

## 2. Ist-Analyse & Plattform-Kopplungen

| Bereich | Windows (Ist) | Linux (Soll) |
|---|---|---|
| **Build-Target** | `.cargo/config.toml` erzwingt `x86_64-pc-windows-msvc` | `.cargo/config.toml` entfernen/neutralisieren; plattformnatives Kompilieren |
| **Abhängigkeiten** | `windows = 0.62`, `winresource = 0.1` global in `Cargo.toml` | Nur für `target.'cfg(windows)'` aktivieren |
| **Build-Skript** | `build.rs` nutzt `winresource` ungeschützt | `winresource` nur unter Windows ausführen |
| **GUI-Backend (eframe)** | `features = ["glow"]`, `default-features = false` | Auf Linux Backends `wayland`, `x11` und `default_fonts` aktivieren |
| **Event-Quelle** | Windows Event Log API (`6005`, `6006`, Kernel-Power, Troubleshooter) | `systemd-journald` (`journalctl --list-boots -o json` und Sleep-Message-IDs) |
| **Fonts** | Hartkodierter Pfad `C:\Windows\Fonts\segoeui.ttf` | Linux: System-Fonts (z. B. DejaVu/Ubuntu) mit sicherem Fallback auf Egui-Default-Fonts |
| **Datei-Öffner** | Aufruf von `explorer.exe` | Aufruf von `xdg-open` |

## 3. Architektur der Linux-Event-Extraktion

Die Domänen- und Aggregationslogik in `src/eventlog.rs` (`PcSession`, `Segment`, `SegmentKind`, `DaySummary`, Tagesschnitt um Mitternacht, Sleep-Verschmelzung, CSV-Export) ist bereits zu 100 % plattformunabhängig und wird unverändert wiederverwendet.

### 3.1 Boot- und Shutdown-Sessions
Abfrage über `journalctl --list-boots -o json`:
- Liefert ein JSON-Array aller Boot-Vorgänge mit `first_entry` und `last_entry` (Mikrosekunden-Timestamp).
- `index < 0`: Beendete Session (`boot_time` = first, `shutdown_time` = Some(last)).
- `index == 0`: Aktuell laufende Session (`boot_time` = first, `shutdown_time` = None).

### 3.2 Sleep- und Hibernate-Phasen
Abfrage über `journalctl -o json` mit systemd-Standard-Message-IDs:
- `MESSAGE_ID=6bbd95ee977941e497c48be27c254128` (Sleep Start): Feld `SLEEP` enthält `"suspend"` bzw. `"hibernate"`.
- `MESSAGE_ID=8811e6df2a8e40f58a94cea26f8ebf14` (Sleep Stop / Resume).
- Zeitstempel über `__REALTIME_TIMESTAMP` (Mikrosekunden-Timestamp).
- Mapping auf bestehende `EventKind::EnteringSleep(SegmentKind)` und `EventKind::Resumed`.

### 3.3 Strukturierung der Module
- `src/main.rs`
- `src/egui_ui.rs`
- `src/eventlog/mod.rs` (oder `src/eventlog.rs`)
- `src/eventlog/windows.rs`
- `src/eventlog/linux.rs`

## 4. Test-Driven Development (TDD) Vorgehen

1. **Unit-Tests mit Fixtures:**
   - Test für `parse_journal_boots`: Parsing der JSON-Ausgabe von `journalctl --list-boots -o json`.
   - Test für `parse_journal_sleeps`: Parsing der JSON-Zeilen von `journalctl` Sleep-/Resume-Events.
2. **Integration in die Aggregation:**
   - Verifikation mit Test-Eventfolgen über `build_daily_summaries` und `pair_events_to_sessions`.
3. **Plattformtests:**
   - Test für `system_font_path` bzw. Font-Fallback.
   - Test für Opener-Befehl (`explorer.exe` vs `xdg-open`).

## 5. Phasierter Umsetzungsplan

- **Phase 1: Build & Abhängigkeiten bereinigen**
  - `.cargo/config.toml` neutralisieren.
  - `Cargo.toml` um `target.'cfg(windows)'` und Linux-Features für `eframe` ergänzen.
  - `build.rs` absichern.
- **Phase 2: Linux Eventlog-Backend per TDD**
  - Modulaufteilung `src/eventlog/`.
  - TDD-Tests mit JSON-Fixtures implementieren.
  - `linux.rs` implementieren (Abfrage per `Command::new("journalctl")`).
- **Phase 3: Egui-UI & Desktop-Integration**
  - Font-Konfiguration resilient gestalten (Egui-Defaults bei fehlender System-Schriftart).
  - Dateimanager-Öffnung auf `xdg-open` unter Linux umstellen.
- **Phase 4: Verifikation & Dokumentation**
  - `cargo test` auf Linux durchführen.
  - UI-Funktionstests unter X11 / Wayland.
  - `README.md` und `MEMORY.md` aktualisieren.
