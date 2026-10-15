//! Multi-platform event-log access and uptime aggregation.
//!
//! Event logs from the OS represent session start and end. The module pairs
//! ordered events, preserves an unmatched latest start as an active session,
//! and splits sessions across midnight for daily totals. Power events mark
//! sleep and hibernate periods, which are excluded from the daily work time.

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use std::collections::BTreeMap;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::query_event_log;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux::query_event_log;

#[cfg(target_os = "macos")]
use self::macos::query_event_log;
#[cfg(any(target_os = "macos", test))]
mod macos;

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn query_event_log(_days: i64) -> Vec<EventEntry> {
    Vec::new()
}

/// A single PC session (boot → shutdown).
#[derive(Debug, Clone, Default)]
pub struct PcSession {
    pub boot_time: DateTime<Local>,
    pub shutdown_time: Option<DateTime<Local>>,
}

impl PcSession {
    /// Duration in seconds. If no shutdown, uses `now` as end.
    pub fn duration_secs(&self) -> i64 {
        let end = self.shutdown_time.unwrap_or_else(Local::now);
        (end - self.boot_time).num_seconds().max(0)
    }
}

/// State of the PC during a [`Segment`]; ordered by how "off" the PC is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SegmentKind {
    #[default]
    Active,
    Sleep,
    Hibernate,
}

/// A continuous time range in which the PC was in one [`SegmentKind`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Segment {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub kind: SegmentKind,
}

impl Segment {
    pub fn duration_secs(&self) -> i64 {
        (self.end - self.start).num_seconds().max(0)
    }
}

/// Daily summary of PC active time.
#[derive(Debug, Clone, Default)]
pub struct DaySummary {
    pub date: NaiveDate,
    /// Awake time, i.e. session time without sleep/hibernate.
    pub total_seconds: i64,
    pub sleep_seconds: i64,
    pub sessions: Vec<PcSession>,
    /// Active and sleep segments of this day, oldest first.
    pub segments: Vec<Segment>,
}

/// Reads PC start and shutdown events.
/// Returns sessions paired from consecutive start→shutdown events.
pub fn read_pc_sessions(days: i64) -> Vec<PcSession> {
    let events = query_event_log(days);
    pair_events_to_sessions(&events)
}

/// Returns daily summaries for the last N days, newest first.
pub fn daily_summaries(days: i64) -> Vec<DaySummary> {
    let events = query_event_log(days);
    let sessions = pair_events_to_sessions(&events);
    let sleeps = sleep_periods(&events);
    build_daily_summaries(&sessions, &sleeps, Local::now(), days)
}

fn build_daily_summaries(
    sessions: &[PcSession],
    sleeps: &[Segment],
    now: DateTime<Local>,
    days: i64,
) -> Vec<DaySummary> {
    let today = now.date_naive();
    let empty_day = |date| DaySummary {
        date,
        ..Default::default()
    };

    let mut per_day: BTreeMap<NaiveDate, DaySummary> = (0..days)
        .map(|offset| today - chrono::Duration::days(offset))
        .map(|date| (date, empty_day(date)))
        .collect();

    for session in sessions {
        let session_end = session.shutdown_time.unwrap_or(now);
        let parts = split_by_day(session.boot_time, session_end);
        let last_index = parts.len().saturating_sub(1);
        for (index, (date, start, end)) in parts.into_iter().enumerate() {
            let is_open = index == last_index && session.shutdown_time.is_none();
            per_day
                .entry(date)
                .or_insert_with(|| empty_day(date))
                .sessions
                .push(PcSession {
                    boot_time: start,
                    shutdown_time: (!is_open).then_some(end),
                });
        }

        for segment in session_segments(session.boot_time, session_end, sleeps) {
            for (date, start, end) in split_by_day(segment.start, segment.end) {
                let part = Segment {
                    start,
                    end,
                    kind: segment.kind,
                };
                if part.duration_secs() == 0 {
                    continue;
                }
                let day = per_day.entry(date).or_insert_with(|| empty_day(date));
                match part.kind {
                    SegmentKind::Active => day.total_seconds += part.duration_secs(),
                    SegmentKind::Sleep | SegmentKind::Hibernate => {
                        day.sleep_seconds += part.duration_secs()
                    }
                }
                day.segments.push(part);
            }
        }
    }

    let mut summaries: Vec<DaySummary> = per_day.into_values().rev().collect();
    for summary in &mut summaries {
        summary.segments.sort_by_key(|segment| segment.start);
    }
    summaries
}

/// Splits a time range at midnight; a day part ends at 23:59:59.
fn split_by_day(
    start: DateTime<Local>,
    end: DateTime<Local>,
) -> Vec<(NaiveDate, DateTime<Local>, DateTime<Local>)> {
    let mut parts = Vec::new();
    let mut current = start;

    while current.date_naive() < end.date_naive() {
        let date = current.date_naive();
        let day_end = local_time(date, 23, 59, 59).unwrap_or(end);
        parts.push((date, current, day_end));
        current = date
            .succ_opt()
            .and_then(|next| local_time(next, 0, 0, 0))
            .unwrap_or(end);
    }
    if end >= current {
        parts.push((current.date_naive(), current, end));
    }
    parts
}

fn local_time(date: NaiveDate, hour: u32, minute: u32, second: u32) -> Option<DateTime<Local>> {
    Local
        .from_local_datetime(&date.and_hms_opt(hour, minute, second)?)
        .earliest()
}

/// Splits a session into active parts and the (merged) sleep periods within it.
fn session_segments(
    start: DateTime<Local>,
    end: DateTime<Local>,
    sleeps: &[Segment],
) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut cursor = start;

    for sleep in sleeps
        .iter()
        .filter(|sleep| sleep.end > start && sleep.start < end)
    {
        let sleep_start = sleep.start.max(cursor);
        let sleep_end = sleep.end.min(end);
        if sleep_start > cursor {
            segments.push(Segment {
                start: cursor,
                end: sleep_start,
                kind: SegmentKind::Active,
            });
        }
        if sleep_end > sleep_start {
            segments.push(Segment {
                start: sleep_start,
                end: sleep_end,
                kind: sleep.kind,
            });
        }
        cursor = cursor.max(sleep_end);
    }
    if end > cursor {
        segments.push(Segment {
            start: cursor,
            end,
            kind: SegmentKind::Active,
        });
    }
    segments
}

/// Derives sorted, non-overlapping sleep/hibernate periods from power events.
fn sleep_periods(events: &[EventEntry]) -> Vec<Segment> {
    let mut periods = Vec::new();
    let mut sleep_start: Option<(DateTime<Local>, SegmentKind)> = None;
    // Kind of the last sleep entry; Power-Troubleshooter events carry no kind.
    let mut sleep_kind = SegmentKind::Sleep;
    let mut standby_start: Option<DateTime<Local>> = None;
    let mut push = |start: DateTime<Local>, end: DateTime<Local>, kind| {
        if end > start {
            periods.push(Segment { start, end, kind });
        }
    };

    for event in events {
        match event.kind {
            EventKind::EnteringSleep(kind) => {
                sleep_start = Some((event.time, kind));
                sleep_kind = kind;
            }
            EventKind::Resumed => {
                if let Some((start, kind)) = sleep_start.take() {
                    push(start, event.time, kind);
                }
            }
            EventKind::ResumedFromLowPower { sleep, wake } => {
                sleep_start = None;
                push(sleep, wake, sleep_kind);
                sleep_kind = SegmentKind::Sleep;
            }
            EventKind::EnteringStandby => {
                standby_start.get_or_insert(event.time);
            }
            EventKind::ExitingStandby => {
                if let Some(start) = standby_start.take() {
                    push(start, event.time, SegmentKind::Sleep);
                }
            }
            EventKind::Boot => {
                // The PC never resumed (e.g. power loss), so it slept until this boot.
                if let Some((start, kind)) = sleep_start.take() {
                    push(start, event.time, kind);
                }
                if let Some(start) = standby_start.take() {
                    push(start, event.time, SegmentKind::Sleep);
                }
            }
            EventKind::Shutdown => {
                sleep_start = None;
                standby_start = None;
            }
        }
    }

    merge_periods(periods)
}

fn merge_periods(mut periods: Vec<Segment>) -> Vec<Segment> {
    periods.sort_by_key(|period| period.start);
    let mut merged: Vec<Segment> = Vec::with_capacity(periods.len());
    for period in periods {
        match merged.last_mut() {
            Some(last) if period.start <= last.end => {
                last.end = last.end.max(period.end);
                last.kind = last.kind.max(period.kind);
            }
            _ => merged.push(period),
        }
    }
    merged
}

/// Generates CSV export string.
pub fn export_csv(days: i64) -> String {
    let sessions = read_pc_sessions(days);
    let mut csv = String::from("Datum;Start;Ende;Dauer\n");
    for s in &sessions {
        let start = s.boot_time.format("%Y-%m-%d %H:%M:%S");
        let end = s
            .shutdown_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "(aktiv)".to_string());
        let dur = s.duration_secs();
        let hours = dur / 3600;
        let mins = (dur % 3600) / 60;
        csv.push_str(&format!(
            "{};{};{};{}h {}m\n",
            s.boot_time.format("%Y-%m-%d"),
            start,
            end,
            hours,
            mins
        ));
    }
    csv
}

// ── Internal ──

#[derive(Debug, Clone, Default)]
pub(crate) struct EventEntry {
    pub(crate) time: DateTime<Local>,
    pub(crate) kind: EventKind,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum EventKind {
    /// Boot / System start
    #[default]
    Boot,
    /// Shutdown / System halt
    Shutdown,
    /// Entering sleep or hibernate
    EnteringSleep(SegmentKind),
    /// System resumed
    Resumed,
    /// Detailed resume with explicit sleep and wake timestamps
    ResumedFromLowPower {
        sleep: DateTime<Local>,
        wake: DateTime<Local>,
    },
    /// Entering Modern Standby
    EnteringStandby,
    /// Exiting Modern Standby
    ExitingStandby,
}

/// Pairs boot → shutdown events into sessions.
fn pair_events_to_sessions(events: &[EventEntry]) -> Vec<PcSession> {
    let mut sessions = Vec::new();
    let mut last_boot: Option<DateTime<Local>> = None;

    for event in events {
        match event.kind {
            EventKind::Boot => {
                if let Some(boot) = last_boot.take() {
                    sessions.push(PcSession {
                        boot_time: boot,
                        shutdown_time: Some(event.time),
                    });
                }
                last_boot = Some(event.time);
            }
            EventKind::Shutdown => {
                if let Some(boot) = last_boot.take() {
                    sessions.push(PcSession {
                        boot_time: boot,
                        shutdown_time: Some(event.time),
                    });
                }
            }
            _ => {}
        }
    }

    if let Some(boot) = last_boot {
        sessions.push(PcSession {
            boot_time: boot,
            shutdown_time: None, // still running
        });
    }

    // Sort newest first
    sessions.sort_by_key(|s| std::cmp::Reverse(s.boot_time));
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 9, day, hour, minute, 0)
            .single()
            .expect("valid test time")
    }

    fn event(time: DateTime<Local>, kind: EventKind) -> EventEntry {
        EventEntry { time, kind }
    }

    #[test]
    fn builds_sleep_period_with_kind_from_preceding_entry_event() {
        let events = [
            event(
                at(24, 12, 0),
                EventKind::EnteringSleep(SegmentKind::Hibernate),
            ),
            event(
                at(24, 13, 1),
                EventKind::ResumedFromLowPower {
                    sleep: at(24, 12, 0),
                    wake: at(24, 13, 0),
                },
            ),
        ];

        assert_eq!(
            sleep_periods(&events),
            vec![Segment {
                start: at(24, 12, 0),
                end: at(24, 13, 0),
                kind: SegmentKind::Hibernate,
            }]
        );
    }

    #[test]
    fn merges_overlapping_resume_events_into_one_period() {
        let events = [
            event(at(24, 12, 0), EventKind::EnteringSleep(SegmentKind::Sleep)),
            event(at(24, 12, 31), EventKind::Resumed),
            event(
                at(24, 12, 32),
                EventKind::ResumedFromLowPower {
                    sleep: at(24, 12, 1),
                    wake: at(24, 12, 30),
                },
            ),
        ];

        assert_eq!(
            sleep_periods(&events),
            vec![Segment {
                start: at(24, 12, 0),
                end: at(24, 12, 31),
                kind: SegmentKind::Sleep,
            }]
        );
    }

    #[test]
    fn closes_unresumed_sleep_at_next_boot() {
        let events = [
            event(at(24, 18, 0), EventKind::EnteringSleep(SegmentKind::Sleep)),
            event(at(25, 8, 0), EventKind::Boot),
        ];

        assert_eq!(
            sleep_periods(&events),
            vec![Segment {
                start: at(24, 18, 0),
                end: at(25, 8, 0),
                kind: SegmentKind::Sleep,
            }]
        );
    }

    #[test]
    fn pairs_modern_standby_events() {
        let events = [
            event(at(24, 12, 0), EventKind::EnteringStandby),
            event(at(24, 12, 45), EventKind::ExitingStandby),
        ];

        assert_eq!(
            sleep_periods(&events),
            vec![Segment {
                start: at(24, 12, 0),
                end: at(24, 12, 45),
                kind: SegmentKind::Sleep,
            }]
        );
    }

    #[test]
    fn splits_session_into_active_and_sleep_segments() {
        let sleeps = [Segment {
            start: at(24, 12, 0),
            end: at(24, 13, 0),
            kind: SegmentKind::Sleep,
        }];

        let segments = session_segments(at(24, 8, 0), at(24, 17, 0), &sleeps);

        let kinds: Vec<_> = segments
            .iter()
            .map(|segment| (segment.start, segment.end, segment.kind))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (at(24, 8, 0), at(24, 12, 0), SegmentKind::Active),
                (at(24, 12, 0), at(24, 13, 0), SegmentKind::Sleep),
                (at(24, 13, 0), at(24, 17, 0), SegmentKind::Active),
            ]
        );
    }

    #[test]
    fn daily_summary_excludes_sleep_from_work_time() {
        let sessions = [PcSession {
            boot_time: at(24, 8, 0),
            shutdown_time: Some(at(24, 17, 0)),
        }];
        let sleeps = [Segment {
            start: at(24, 12, 0),
            end: at(24, 13, 0),
            kind: SegmentKind::Sleep,
        }];

        let summaries = build_daily_summaries(&sessions, &sleeps, at(25, 9, 0), 2);

        let day = summaries
            .iter()
            .find(|summary| summary.date == at(24, 0, 0).date_naive())
            .expect("summary for 24th");
        assert_eq!(day.total_seconds, 8 * 3_600);
        assert_eq!(day.sleep_seconds, 3_600);
        assert_eq!(day.segments.len(), 3);
        assert_eq!(summaries[0].date, at(25, 0, 0).date_naive());
    }

    #[test]
    fn splits_sleep_across_midnight() {
        let sessions = [PcSession {
            boot_time: at(24, 20, 0),
            shutdown_time: None,
        }];
        let sleeps = [Segment {
            start: at(24, 22, 0),
            end: at(25, 7, 0),
            kind: SegmentKind::Sleep,
        }];

        let summaries = build_daily_summaries(&sessions, &sleeps, at(25, 9, 0), 2);

        let today = &summaries[0];
        assert_eq!(today.total_seconds, 2 * 3_600);
        assert_eq!(today.sleep_seconds, 7 * 3_600);
        assert_eq!(today.sessions[0].shutdown_time, None);
    }
}
