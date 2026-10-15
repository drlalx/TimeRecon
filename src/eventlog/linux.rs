use chrono::{DateTime, Local, TimeZone};
use serde_json::Value;
use std::process::Command;

use super::{EventEntry, EventKind, SegmentKind};

const SLEEP_START_MESSAGE_ID: &str = "6bbd95ee977941e497c48be27c254128";
const SLEEP_STOP_MESSAGE_ID: &str = "8811e6df2a8e40f58a94cea26f8ebf14";

/// Queries systemd journal for boot/shutdown and sleep/resume events.
pub(crate) fn query_event_log(days: i64) -> Vec<EventEntry> {
    let mut entries = Vec::new();

    // 1. Query boot and shutdown sessions
    if let Some(output) = Command::new("journalctl")
        .args(["--list-boots", "-o", "json"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let json_str = std::str::from_utf8(&output.stdout).unwrap_or_default();
        entries.extend(parse_boots_json(json_str));
    }

    // 2. Query sleep and resume events
    let since_arg = format!("{days} days ago");
    let start_match = format!("MESSAGE_ID={SLEEP_START_MESSAGE_ID}");
    let stop_match = format!("MESSAGE_ID={SLEEP_STOP_MESSAGE_ID}");

    if let Some(output) = Command::new("journalctl")
        .args([
            "-o",
            "json",
            &start_match,
            "+",
            &stop_match,
            "--since",
            &since_arg,
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let lines_str = std::str::from_utf8(&output.stdout).unwrap_or_default();
        entries.extend(parse_sleep_lines(lines_str));
    }

    // Sort all events chronologically
    entries.sort_by_key(|entry| entry.time);
    entries
}

/// Parses the output of `journalctl --list-boots -o json`.
pub(crate) fn parse_boots_json(json_str: &str) -> Vec<EventEntry> {
    let Ok(boots) = serde_json::from_str::<Vec<Value>>(json_str) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for boot in boots {
        let Some(first_micros) = boot.get("first_entry").and_then(Value::as_i64) else {
            continue;
        };
        let Some(boot_time) = timestamp_from_micros(first_micros) else {
            continue;
        };

        entries.push(EventEntry {
            time: boot_time,
            kind: EventKind::Boot,
        });

        let index = boot.get("index").and_then(Value::as_i64).unwrap_or(0);
        // index == 0 is the current ongoing boot session (not yet shut down)
        if index < 0 {
            let shutdown_time = boot
                .get("last_entry")
                .and_then(Value::as_i64)
                .and_then(timestamp_from_micros);
            if let Some(time) = shutdown_time {
                entries.push(EventEntry {
                    time,
                    kind: EventKind::Shutdown,
                });
            }
        }
    }

    entries
}

/// Parses newline-delimited JSON events for sleep and resume operations.
pub(crate) fn parse_sleep_lines(lines_str: &str) -> Vec<EventEntry> {
    let mut entries = Vec::new();

    for line in lines_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(obj) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        let message_id = obj.get("MESSAGE_ID").and_then(Value::as_str).unwrap_or("");
        let timestamp_micros = obj
            .get("__REALTIME_TIMESTAMP")
            .and_then(|val| {
                val.as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .or_else(|| val.as_i64())
            });

        let Some(micros) = timestamp_micros else {
            continue;
        };
        let Some(time) = timestamp_from_micros(micros) else {
            continue;
        };

        match message_id {
            SLEEP_START_MESSAGE_ID => {
                let sleep_op = obj.get("SLEEP").and_then(Value::as_str).unwrap_or("suspend");
                let kind = if sleep_op.eq_ignore_ascii_case("hibernate") {
                    SegmentKind::Hibernate
                } else {
                    SegmentKind::Sleep
                };
                entries.push(EventEntry {
                    time,
                    kind: EventKind::EnteringSleep(kind),
                });
            }
            SLEEP_STOP_MESSAGE_ID => {
                entries.push(EventEntry {
                    time,
                    kind: EventKind::Resumed,
                });
            }
            _ => {}
        }
    }

    entries
}

fn timestamp_from_micros(micros: i64) -> Option<DateTime<Local>> {
    let secs = micros / 1_000_000;
    let nsecs = ((micros % 1_000_000) * 1_000) as u32;
    Local.timestamp_opt(secs, nsecs).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eventlog::daily_summaries;

    #[test]
    fn parses_journalctl_boots_json_correctly() {
        let json = r#"[
            {"index": -1, "boot_id": "abc1", "first_entry": 1788092235000000, "last_entry": 1788093192000000},
            {"index": 0, "boot_id": "abc2", "first_entry": 1788094189000000, "last_entry": 1792007341000000}
        ]"#;

        let entries = parse_boots_json(json);
        assert_eq!(entries.len(), 3);

        // Previous boot: boot + shutdown
        assert_eq!(entries[0].kind, EventKind::Boot);
        assert_eq!(entries[1].kind, EventKind::Shutdown);

        // Current boot: boot only
        assert_eq!(entries[2].kind, EventKind::Boot);
    }

    #[test]
    fn parses_journalctl_sleep_json_lines_correctly() {
        let lines = r#"
{"MESSAGE_ID":"6bbd95ee977941e497c48be27c254128","SLEEP":"suspend","__REALTIME_TIMESTAMP":"1791651982777817"}
{"MESSAGE_ID":"8811e6df2a8e40f58a94cea26f8ebf14","SLEEP":"suspend","__REALTIME_TIMESTAMP":"1791712753673772"}
{"MESSAGE_ID":"6bbd95ee977941e497c48be27c254128","SLEEP":"hibernate","__REALTIME_TIMESTAMP":"1791750952053364"}
{"MESSAGE_ID":"8811e6df2a8e40f58a94cea26f8ebf14","SLEEP":"hibernate","__REALTIME_TIMESTAMP":"1791801517774656"}
"#;

        let entries = parse_sleep_lines(lines);
        assert_eq!(entries.len(), 4);

        assert_eq!(entries[0].kind, EventKind::EnteringSleep(SegmentKind::Sleep));
        assert_eq!(entries[1].kind, EventKind::Resumed);
        assert_eq!(entries[2].kind, EventKind::EnteringSleep(SegmentKind::Hibernate));
        assert_eq!(entries[3].kind, EventKind::Resumed);
    }

    #[test]
    fn handles_malformed_json_gracefully() {
        assert!(parse_boots_json("").is_empty());
        assert!(parse_boots_json("{not an array}").is_empty());
        assert!(parse_sleep_lines("").is_empty());
        assert!(parse_sleep_lines("invalid json line\n{\"incomplete\": 1}").is_empty());
    }

    #[test]
    fn queries_local_systemd_journal_without_crashing() {
        let entries = query_event_log(14);
        for window in entries.windows(2) {
            assert!(window[0].time <= window[1].time);
        }

        let summaries = daily_summaries(14);
        let total_sleep: i64 = summaries.iter().map(|d| d.sleep_seconds).sum();
        eprintln!("Total sleep seconds detected in last 14 days: {total_sleep}");
    }
}
