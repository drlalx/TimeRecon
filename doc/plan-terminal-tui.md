# Plan: Terminal-TUI für TimeRecon

## 1. Ziel und Vision

Neben der grafischen Egui-Oberfläche soll TimeRecon eine vollwertige, interaktive Terminal-Benutzeroberfläche (**TUI**) erhalten. Dies ermöglicht die Nutzung über SSH, in reinen Server-/Headless-Umgebungen, in tmux/zellij und für Anwender, die terminalbasierte Workflows bevorzugen.

Zusätzlich soll ein Headless-Modus (`--status`, `--export`) bereitgestellt werden, der sich ideal für Shell-Prompts, Statusleisten (z. B. Waybar, i3status) und CI/Skripte eignet.

---

## 2. CLI-Modusauswahl und Integration

TimeRecon soll über Befehlszeilenparameter steuerbar sein:

| Argument | Verhalten |
| :--- | :--- |
| *(keine Argumente)* | Startet Egui-GUI, falls eine grafische Sitzung (`DISPLAY` / `WAYLAND_DISPLAY`) vorhanden ist; ansonsten automatischer Fallback auf TUI. |
| `--tui` / `-t` | Erzwingt den Start der interaktiven Ratatui-Terminal-Oberfläche. |
| `--gui` / `-g` | Erzwingt den Start der grafischen Egui-Oberfläche (Fehler, falls kein Display). |
| `--status` / `-s` | Gibt eine einzeilige Statusmeldung auf `stdout` aus (z. B. `● On | Today: 4h 12m | Month: 82h 15m`). Ideal für Statusbars. |
| `--export [PFAD]` | Erstellt direkt einen CSV-Export ohne Benutzeroberfläche und beendet das Programm mit Exit-Code 0. |
| `--version` / `-v` | Gibt die Versionsnummer (`TimeRecon 0.1.0`) aus. |
| `--help` / `-h` | Zeigt Hilfe und Tastenkombinationen an. |

---

## 3. Bibliotheken und Lizenz-Konformität

Analog zum Schwesterprojekt `OxiDisc` setzen wir auf moderne, performante und bewährte Crates:

```toml
[features]
default = ["egui-ui", "tui"]
egui-ui = ["dep:eframe"]
tui = ["dep:ratatui", "dep:crossterm"]

[dependencies]
ratatui = { version = "0.30", default-features = false, features = ["crossterm"], optional = true }
crossterm = { version = "0.29", optional = true }
```

> [!NOTE]
> Sowohl `ratatui` (MIT) als auch `crossterm` (MIT) besitzen eine uneingeschränkte MIT-Lizenz. Die strikte Lizenz-Richtlinie in `deny.toml` sowie der Test `tests/test_license_compatibility.rs` bleiben damit zu 100 % erfüllt.

---

## 4. TUI-Architektur und Modulaufbau

```
src/
├── main.rs                 # CLI-Argument-Parsing & Backend-Auswahl (GUI vs. TUI vs. Headless)
├── eventlog/               # Plattformunabhängige Log-Aggregation (Linux, Windows, macOS)
├── egui_ui.rs              # Grafische Oberfläche (feature = "egui-ui")
└── tui/                    # Terminal-Oberfläche (feature = "tui")
    ├── mod.rs              # Terminal-Initialisierung, Cleanup-Guard, Event-Loop
    ├── app.rs              # TUI-State, Navigation, Filter & Aktionen
    ├── timeline.rs         # Unicode-24h-Zeitstrahl-Generator (getestet via Unit-Tests)
    └── view.rs             # Ratatui-Widgets, Layout, Farben & Tabs
```

### Terminal Cleanup Guard (Panic-Sicherheit)
Ein RAII-Guard stellt sicher, dass das Terminal (Alternate Screen, Mouse Capture, Raw Mode) bei jedem Beenden oder bei unvorhergesehenen Fehlern zuverlässig in den Ursprungszustand zurückversetzt wird.

---

## 5. UI-Konzept und Visualisierung

### Layout-Skizze (80x24 Standard-Terminal)

```
┌─ TimeRecon v0.1.0 ────────────────────────────────────────── [● PC on] ─┐
│ Today: 04h 12m (Active) | Sleep: 01h 05m | Span: 08:30 → now            │
│ Month: 82h 15m | Daily Avg: 06h 51m | Active Days: 12                   │
├─────────────────────────────────────────────────────────────────────────┤
│ [1: Daily Overview]  2: Sessions                                        │
│                                                                         │
│ Date        Work     Sleep    Span         00 03 06 09 12 15 18 21 24   │
│ ─────────────────────────────────────────────────────────────────────── │
│ > Today     04h 12m  01h 05m  08:30 → now  ·········████░░████········· │
│   Mi 14.10  08h 45m  02h 10m  07:15 → 18:10 ········███░░░██████······· │
│   Di 13.10  07h 20m  00h 00m  08:00 → 15:20 ········███████············ │
│   Mo 12.10  09h 10m  01h 30m  08:10 → 18:50 ········████░░█████········ │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│ Legend: [█] Active  [░] Sleep (Suspend)  [▓] Hibernate  [·] Off         │
│ Controls: [q] Quit  [r] Refresh  [e] Export CSV  [Tab] Switch Tab  [↑↓] │
└─────────────────────────────────────────────────────────────────────────┘
```

### Visualisierungselemente
- **24-Stunden-Zeitstrahl:**  
  Je nach Terminalbreite dynamisch skaliert (z. B. 24 Zeichen = 1h/Zeichen, 48 Zeichen = 30min/Zeichen).
  - `█` (Cyan/Blau): Aktive Arbeitszeit (PC wach).
  - `░` (Gelb/Orange): Sleep / Suspend (S3 / Modern Standby).
  - `▓` (Magenta): Hibernate (Ruhezustand).
  - `·` (Dunkelgrau): PC Ausgeschaltet.
- **Sessions-Tab:**  
  Detailtabelle aller Einzelsitzungen von Boot bis Shutdown für den ausgewählten Tag mit Start, Ende, Dauer und Status.

---

## 6. Tastatur- und Maussteuerung

| Taste | Funktion |
| :--- | :--- |
| `q` / `Esc` / `Ctrl+C` | TUI sicher beenden und Terminal zurücksetzen |
| `Tab` / `1` / `2` | Zwischen Übersicht (Timeline) und Einzelsitzungen umschalten |
| `↑` / `k` bzw. `↓` / `j` | Tag auswählen / durch die Tage scrollen |
| `r` / `F5` | Daten aus den System-Logs neu laden |
| `e` | CSV-Export im aktuellen oder Documents-Verzeichnis speichern |
| `?` | Hilfe-Overlay ein-/ausblenden |

---

## 7. Testgetriebene Umsetzung (TDD)

Gemäß Entwicklungsleitlinie wird neue Logik testgetrieben implementiert:
1. **`src/tui/timeline.rs` Tests:**
   - Diskretisierung von Start- und Endzeiten in 24/48-Spalten-Raster.
   - Korrekte Überlagerung und Priorisierung (Hibernate > Sleep > Active).
   - Formatierung von Wochentagen, Datumsstrings und Zeitspannen.
2. **Clippy-Konformität:**
   - Strikte No-Panic-Regeln (`unwrap_used = "deny"`, `expect_used = "deny"`).
   - Saubere Fehlerbehandlung in I/O und Crossterm-Aufrufen.
3. **Lizenz-Kompatibilitäts-Test:**
   - Automatisierter `cargo test` stellt sicher, dass alle neuen Abhängigkeiten von `ratatui` und `crossterm` MIT-kompatibel sind.

---

## 8. Phasenplan (Atomic Commits)

1. **Phase 1: Feature-Flags & CLI-Parsing**
   - Ergänzung von `Cargo.toml` um die optionalen Features `egui-ui` und `tui`.
   - CLI-Argument-Parser in `src/main.rs` (`--tui`, `--gui`, `--status`, `--export`).
2. **Phase 2: Timeline-Generator & Tests (TDD)**
   - Implementierung von `src/tui/timeline.rs` mit umfassenden Unit-Tests für Unicode-Blocks.
3. **Phase 3: Ratatui TUI Rendering & Navigation**
   - Implementierung von `src/tui/mod.rs`, `app.rs` und `view.rs`.
   - Einbindung der Tastensteuerung und Tabs.
4. **Phase 4: Headless Modus**
   - Einzeilen-Ausgabe (`--status`) und Headless-CSV-Export (`--export`).
5. **Phase 5: Dokumentation & Verifikation**
   - Aktualisierung von `README.md` und `doc/README.md`.
   - Screenshot / Terminal-Aufnahme der neuen TUI.
