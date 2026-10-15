# Cross-Platform Spezifikation & Plan: `active-time` (Windows, Linux & macOS)

## 1. Ziel und Vision

`active-time` soll als schlankes, privatsphärefreundliches Tool auf allen gängigen Desktop-Plattformen (**Windows**, **Linux** und **macOS**) laufen.
Das Kernprinzip bleibt unverändert: **Kein dauerhafter Hintergrund-Daemon, keine Tasten- oder Fensterspionage**. Stattdessen wird die tatsächliche Rechner-Nutzungszeit (Boot bis Shutdown abzüglich Sleep/Hibernate/Standby) beim Start bzw. Refresh aus den standardmäßigen Betriebssystem-Protokollen rekonstruiert.

---

## 2. Plattform-Matrix im Überblick

| Komponente | Windows | Linux | macOS (iMac / MacBook) |
|---|---|---|---|
| **OS-Eventquelle (Boots)** | Windows Event Log (IDs 6005/6006) | `systemd-journald` (`journalctl --list-boots -o json`) | `last reboot` / `sysctl kern.boottime` / Unified Log |
| **OS-Eventquelle (Sleep/Wake)** | Kernel-Power 42/107/506/507, Power-Troubleshooter 1 | `systemd-sleep` (Message-IDs `6bbd9...` & `8811e...`) | `pmset -g log` / Unified Log (`powerd`) |
| **Abfrage-Mechanismus** | Win32 C-API (`windows` Crate) | CLI-JSON-Stream (`journalctl`) | CLI / Unified Log (`pmset` / `log show`) |
| **GUI-Backends** | `eframe` (Glow/WGL) / Slint | `eframe` (Glow, Wayland/X11) / Slint | `eframe` (Glow, Cocoa) / Slint |
| **Standard-Dateimanager** | `explorer.exe <path>` | `xdg-open <path>` | `open <path>` |
| **Standard-Schriftart** | `Segoe UI` (`C:\Windows\Fonts`) | DejaVu Sans / Ubuntu / Egui Default | SF Pro / Helvetica / Egui Default |

---

## 3. Architektur der plattformunabhängigen Abstraktion

### 3.1 Unveränderter Domänenkern (`src/eventlog/mod.rs`)
Die Domänen- und Aggregationslogik ist vollständig betriebssystemagnostisch und wird zu 100 % geteilt:
- Datenstrukturen: `PcSession`, `DaySummary`, `Segment`, `SegmentKind`
- Algorithmen: `split_by_day` (Mitternachtsaufteilung), `session_segments`, `merge_periods`, `pair_events_to_sessions`, `export_csv`

### 3.2 Einheitlicher Adapter-Vertrag
Jeder plattformspezifische Provider implementiert lediglich eine interne Funktion:
```rust
pub(crate) fn query_event_log(days: i64) -> Vec<EventEntry>
```
mit dem gemeinsamen Ereignistyp:
```rust
pub(crate) struct EventEntry {
    pub(crate) time: DateTime<Local>,
    pub(crate) kind: EventKind,
}

pub(crate) enum EventKind {
    Boot,
    Shutdown,
    EnteringSleep(SegmentKind),
    Resumed,
    ResumedFromLowPower { sleep: DateTime<Local>, wake: DateTime<Local> },
    EnteringStandby,
    ExitingStandby,
}
```

### 3.3 Verzeichnis- & Modulstruktur
- `src/main.rs`
- `src/egui_ui.rs`
- `src/eventlog/mod.rs` (oder `src/eventlog.rs`)
- `src/eventlog/windows.rs`
- `src/eventlog/linux.rs`
- `src/eventlog/macos.rs`

---

## 4. Plattformspezifische Implementierungsdetails

### 4.1 Windows (bestehende Funktionalität)
- Verwendet weiterhin die native Win32 EventLog-API via Microsoft `windows`-Crate.
- `winresource` bettet das Anwendungs-Icon ein.
- Keine Regressionen oder Verhaltensänderungen.

### 4.2 Linux (Phase 1)
1. **Boots & Sessions:**
   Aufruf von `journalctl --list-boots -o json`:
   - Liefert alle Reboots mit `first_entry` und `last_entry` in Mikrosekunden.
   - Boot mit `index == 0` ist die aktuell laufende Session (`shutdown_time = None`).
   - Ältere Boots (`index < 0`) liefern `shutdown_time = Some(...)`.
2. **Sleep & Hibernate:**
   Aufruf von `journalctl -o json MESSAGE_ID=...`:
   - `6bbd95ee977941e497c48be27c254128` -> Sleep-Start (Sleep oder Hibernate)
   - `8811e6df2a8e40f58a94cea26f8ebf14` -> Sleep-Ende (Resumed)
3. **Robustheit & Fallback:**
   Sollte `journalctl` auf Non-systemd-Systemen oder wegen fehlender Rechte nicht verfügbar sein, fängt das Modul Fehler transparent ab und gibt dem Nutzer einen verständlichen Statushinweis.

### 4.3 macOS (Phase 2 / iMac Vorbereitung)
1. **Boots & Sessions:**
   - `sysctl -n kern.boottime` liefert die aktuelle Boot-Zeit.
   - `last reboot` / `last shutdown` liefert historische Boot/Shutdown-Zeiten.
2. **Sleep & Wake:**
   - `pmset -g log` protokolliert historische Power-Zustandswechsel (`Sleep`, `Wake`, `DarkWake`) mit exakten Datums-/Zeitstempeln.
3. **Opener & Fonts:**
   - Öffnen via `open <pfad>`.
   - Fonts: Egui Default-Fonts oder SF Pro Fallback.

---

## 5. Build-Konfiguration (`Cargo.toml`)

Damit das Projekt auf jeder Plattform ohne manuelle Flag-Tricks nativ baut:
1. **`.cargo/config.toml`:**
   Das hartkodierte Windows-Target wird entfernt, sodass Cargo auf dem jeweiligen Host das native Target wählt.
2. **Plattform-spezifische Dependencies:**
   ```toml
   [target.'cfg(windows)'.dependencies]
   windows = { version = "0.62.2", features = ["Win32_Foundation", "Win32_System_EventLog"] }

   [target.'cfg(windows)'.build-dependencies]
   winresource = "0.1"

   [target.'cfg(windows)'.dependencies.eframe]
   version = "0.33.3"
   default-features = false
   features = ["glow"]
   optional = true

   [target.'cfg(target_os = "linux")'.dependencies.eframe]
   version = "0.33.3"
   default-features = false
   features = ["glow", "wayland", "x11", "default_fonts"]
   optional = true

   [target.'cfg(target_os = "macos")'.dependencies.eframe]
   version = "0.33.3"
   default-features = false
   features = ["glow", "default_fonts"]
   optional = true
   ```

---

## 6. Test-Driven Development (TDD) Vorgehen

1. **Vorab-Unit-Tests für Linux-Log-Parsing:**
   - `test_parse_journalctl_boots_json`: Testet Extraktion von abgeschlossenen und laufenden Sitzungen aus Fixture-Daten.
   - `test_parse_journalctl_sleep_json`: Testet Extraktion von Suspend vs. Hibernate aus JSON-Zeilen.
2. **Vorab-Unit-Tests für Plattform-Helfer:**
   - `test_platform_file_opener`: Verifiziert Befehlsauswahl (`explorer.exe` vs `xdg-open` vs `open`).
   - `test_font_fallback`: Verifiziert, dass fehlende System-Fonts nicht zum Crash führen, sondern sauber fallbacken.
3. **End-to-End Regressions-Tests:**
   - Sicherstellen, dass alle bestehenden Aggregations- und Mitternachts-Tests für alle Zielsysteme grün bleiben.

---

## 7. Phasierter Umsetzungsplan

- **Phase 1: Multi-Platform Build-Setup**
  - Bereinigung von `.cargo/config.toml`, `Cargo.toml` und `build.rs`.
  - Windows-Build-Kompatibilität sicherstellen, Linux-Kompilierung aktivieren.
- **Phase 2: Linux-Backend per TDD umsetzen**
  - Aufteilung von `src/eventlog/` in Module (`mod.rs`, `windows.rs`, `linux.rs`).
  - Unit-Tests mit JSON-Fixtures schreiben.
  - Linux `journalctl`-Abfrage implementieren.
- **Phase 3: Egui-UI & Desktop-Integration harmonisieren**
  - Cross-Platform Opener (`xdg-open` / `explorer.exe` / `open`).
  - Robuste Schriftarten-Auswahl mit Default-Fallback.
- **Phase 4: Vorbereitung macOS (iMac)**
  - `src/eventlog/macos.rs` anlegen (Parsing von `pmset -g log` und `last reboot`).
  - Adapter-Schnittstelle verifizieren.
- **Phase 5: Verifikation & Dokumentation**
  - Tests auf Linux ausführen (`cargo test`).
  - Dokumentation in `README.md` und `MEMORY.md` aktualisieren.
