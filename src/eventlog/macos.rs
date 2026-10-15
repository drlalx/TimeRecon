use chrono::{DateTime, Local};
use std::process::Command;

use super::{EventEntry, EventKind, SegmentKind};

/// Queries macOS power logs and system boot history.
#[allow(dead_code)]
pub(crate) fn query_event_log(days: i64) -> Vec<EventEntry> {
    let mut entries = Vec::new();

    // 1. Query sleep and wake events via pmset
    if let Some(output) = Command::new("pmset")
        .args(["-g", "log"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let log_str = std::str::from_utf8(&output.stdout).unwrap_or_default();
        entries.extend(parse_pmset_log(log_str));
    }

    // 2. Query boot times via sysctl kern.boottime (current boot)
    if let Some(output) = Command::new("sysctl")
        .args(["-n", "kern.boottime"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let out_str = std::str::from_utf8(&output.stdout).unwrap_or_default();
        if let Some(boot_entry) = parse_kern_boottime(out_str) {
            entries.push(boot_entry);
        }
    }

    // Filter by requested days
    let cutoff = Local::now() - chrono::Duration::days(days);
    entries.retain(|entry| entry.time >= cutoff);

    entries.sort_by_key(|entry| entry.time);
    entries
}

/// Parses the output of macOS `pmset -g log` for sleep and wake timestamps.
pub(crate) fn parse_pmset_log(log_str: &str) -> Vec<EventEntry> {
    let mut entries = Vec::new();

    for line in log_str.lines() {
        let line = line.trim();
        // Standard pmset log timestamp: "2026-10-14 12:00:00 +0200" (25 chars)
        if line.len() < 25 {
            continue;
        }

        let Ok(parsed_time) = DateTime::parse_from_str(&line[..25], "%Y-%m-%d %H:%M:%S %z") else {
            continue;
        };
        let time = parsed_time.with_timezone(&Local);
        let rest = &line[25..];
        let mut parts = rest.split_whitespace();
        let Some(action) = parts.next() else {
            continue;
        };

        if action == "Sleep" {
            entries.push(EventEntry {
                time,
                kind: EventKind::EnteringSleep(SegmentKind::Sleep),
            });
        } else if action == "Wake" {
            entries.push(EventEntry {
                time,
                kind: EventKind::Resumed,
            });
        } else if action == "Boot" || action == "Start" {
            entries.push(EventEntry {
                time,
                kind: EventKind::Boot,
            });
        } else if action == "Shutdown" || action == "Halt" {
            entries.push(EventEntry {
                time,
                kind: EventKind::Shutdown,
            });
        }
    }

    entries
}

/// Parses `sysctl -n kern.boottime` into a Boot [`EventEntry`].
pub(crate) fn parse_kern_boottime(out_str: &str) -> Option<EventEntry> {
    let sec_str = out_str.split("sec = ").nth(1)?.split([',', '}']).next()?.trim();
    let sec = sec_str.parse::<i64>().ok()?;
    let time = DateTime::from_timestamp(sec, 0)?.with_timezone(&Local);
    Some(EventEntry {
        time,
        kind: EventKind::Boot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pmset_sleep_and_wake_entries() {
        let log = r#"
2026-10-10 08:00:00 +0200 Boot                 Boot
2026-10-10 22:30:15 +0200 Sleep               Entering Sleep state due to 'Idle Sleep': Using AC (Charge:100%)
2026-10-11 07:15:20 +0200 Wake                Wake from Deep Idle [CDNVA] due to EC.RTC/Alarm: Using AC (Charge:100%)
2026-10-11 07:15:21 +0200 DarkWake            DarkWake from Normal Sleep
2026-10-11 20:00:00 +0200 Shutdown            Shutdown
"#;

        let entries = parse_pmset_log(log);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].kind, EventKind::Boot);
        assert_eq!(entries[1].kind, EventKind::EnteringSleep(SegmentKind::Sleep));
        assert_eq!(entries[2].kind, EventKind::Resumed);
        assert_eq!(entries[3].kind, EventKind::Shutdown);
    }

    #[test]
    fn parses_kern_boottime_correctly() {
        let output = "{ sec = 1788094189, usec = 0 } Sun Aug 30 14:49:49 2026";
        let entry = parse_kern_boottime(output).expect("valid boottime");
        assert_eq!(entry.kind, EventKind::Boot);
        assert_eq!(entry.time.timestamp(), 1788094189);
    }

    #[test]
    fn ignores_malformed_pmset_log_lines() {
        let log = "random line without timestamp\n2026-10-10 invalid timestamp";
        assert!(parse_pmset_log(log).is_empty());
        assert!(parse_kern_boottime("invalid sysctl").is_none());
    }
}
