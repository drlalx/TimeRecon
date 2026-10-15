# Plan: Git-Historie komplett neu starten (Clean Reset)

## Zielsetzung
Die bisherige experimentelle Entwicklungshistorie (inklusive der alten Windows/Slint-Zwischenschritte aus `active-time`) soll vollständig zurückgesetzt werden. Das Repository soll als sauberes, professionelles Projekt für **TimeRecon v0.1.0** neu gestartet werden.

---

## 1. Sicherheits-Backup (Zero Data Loss)
Bevor die Historie verworfen wird, erstellen wir eine Sicherung der bisherigen Historie:
1. **Lokaler Archiv-Branch / Tag:**
   ```bash
   git branch backup/legacy-history
   ```
2. **Git-Bundle-Sicherung (optional außerhalb des Repos):**
   ```bash
   git bundle create ../TimeRecon-legacy-history.bundle --all
   ```
Damit ist garantiert, dass frühere Commits und Slint-Referenzen bei Bedarf noch einsehbar sind.

---

## 2. Technisches Vorgehen (Orphan-Branch-Methode)

Statt das gesamte `.git`-Verzeichnis zu löschen, verwenden wir die native und sichere `git checkout --orphan`-Methode:

1. **Neuen elternlosen Branch erstellen:**
   ```bash
   git checkout --orphan main
   ```
   *Vorteil:* Der Arbeitsbereich bleibt 100 % unverändert, aber Git startet mit einem leeren Commit-Baum ohne Vorgänger.

2. **Alte Branches entfernen:**
   ```bash
   git branch -D master
   ```

---

## 3. Strukturierungsoptionen für die neue Historie

### Variante A: Ein einzelner, vollständiger Initial Commit (Empfohlen für GitHub Release)
Alle aktuellen Dateien werden in einem einzigen sauberen Release-Commit bereitgestellt:
- **Commit:** `feat: initial release of TimeRecon v0.1.0`
- **Vorteil:** Maximale Klarheit, exakt wie ein neues Open-Source-Projekt bei der Erstveröffentlichung.

### Variante B: Strukturierte atomare Commits
Die Dateien werden in 4 logischen Schritten aufgebaut:
1. `chore: project initialization, build configuration, and assets`
   - `Cargo.toml`, `.cargo/config.toml`, `.gitignore`, `build.rs`, `build.sh`, `build.bat`, `assets/`
2. `feat(eventlog): multi-platform event log parsing for Linux, Windows, and macOS`
   - `src/eventlog/`
3. `feat(ui): desktop interface with egui, 24h timeline, and sleep visualization`
   - `src/egui_ui.rs`, `src/main.rs`
4. `feat(meta): MIT license, deny.toml policy, tests, and documentation`
   - `LICENSE`, `deny.toml`, `tests/`, `README.md`, `doc/`, `MEMORY.md`, `CLAUDE.md`, `todo.md`

---

## 4. Validierung nach dem Reset

Jeder Schritt wird durch automatisierte Qualitätsprüfungen verifiziert:
```bash
rtk cargo check --all-targets
rtk cargo test
rtk cargo clippy
rtk cargo build --release
```

---

## 5. Rollback-Plan
Sollte die neue Historie aus irgendeinem Grund nicht gefallen, kann jederzeit via:
```bash
git checkout backup/legacy-history
git branch -M backup/legacy-history master
```
auf den vorherigen Stand zurückgesprungen werden.
