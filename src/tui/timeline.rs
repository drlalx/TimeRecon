//! Terminal timeline calculation and formatting.
//!
//! Provides discretization of continuous day segments into terminal cells,
//! with distinct Unicode block representations for active, sleep, and hibernate periods.

use crate::eventlog::{DaySummary, SegmentKind};
use chrono::Timelike;

/// The state of a single cell in the 24-hour terminal timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    /// PC was turned off during this interval.
    Off,
    /// PC was awake and active.
    Active,
    /// PC was sleeping / suspended (S3, modern standby).
    Sleep,
    /// PC was in hibernation (S4).
    Hibernate,
}

impl CellKind {
    /// Returns the character representation for this cell.
    #[allow(dead_code)]
    pub const fn character(self) -> char {
        match self {
            Self::Off => '·',
            Self::Active => '█',
            Self::Sleep => '░',
            Self::Hibernate => '▓',
        }
    }
}

/// Discretizes a [`DaySummary`] into `width` timeline cells.
///
/// Each cell covers `86_400 / width` seconds of the day.
/// Priority order for overlapping intervals: Hibernate > Sleep > Active > Off.
pub fn render_timeline_cells(day: &DaySummary, width: usize) -> Vec<CellKind> {
    if width == 0 {
        return Vec::new();
    }

    let mut cells = vec![CellKind::Off; width];
    let total_day_seconds = 86_400_f64;
    let step = total_day_seconds / (width as f64);

    for (i, cell) in cells.iter_mut().enumerate() {
        let cell_start = (i as f64) * step;
        let cell_end = ((i + 1) as f64) * step;

        let mut kind = CellKind::Off;

        for segment in &day.segments {
            let seg_start = (segment.start.hour() as f64) * 3600.0
                + (segment.start.minute() as f64) * 60.0
                + (segment.start.second() as f64);
            let seg_end = if segment.end.date_naive() > segment.start.date_naive() {
                86_400.0
            } else {
                (segment.end.hour() as f64) * 3600.0
                    + (segment.end.minute() as f64) * 60.0
                    + (segment.end.second() as f64)
            };

            // Check if segment overlaps with the cell interval
            if seg_end > cell_start && seg_start < cell_end {
                let cell_k = match segment.kind {
                    SegmentKind::Active => CellKind::Active,
                    SegmentKind::Sleep => CellKind::Sleep,
                    SegmentKind::Hibernate => CellKind::Hibernate,
                };
                kind = match (kind, cell_k) {
                    (CellKind::Off, other) => other,
                    (CellKind::Active, CellKind::Sleep | CellKind::Hibernate) => cell_k,
                    (CellKind::Sleep, CellKind::Hibernate) => CellKind::Hibernate,
                    (prev, _) => prev,
                };
            }
        }

        *cell = kind;
    }

    cells
}

/// Converts a [`DaySummary`] directly into a timeline string of length `width`.
#[allow(dead_code)]
pub fn render_timeline_string(day: &DaySummary, width: usize) -> String {
    render_timeline_cells(day, width)
        .into_iter()
        .map(CellKind::character)
        .collect()
}

/// Formats duration in seconds into `Xh YYm`.
pub fn format_duration(seconds: i64) -> String {
    let positive = seconds.max(0);
    let hours = positive / 3600;
    let minutes = (positive % 3600) / 60;
    format!("{hours}h {minutes:02}m")
}

/// Returns the formatted time span `HH:MM → HH:MM` (or `now`) for a day.
pub fn format_day_span(day: &DaySummary) -> String {
    if day.sessions.is_empty() {
        return "–".to_string();
    }

    let earliest = day.sessions.iter().map(|s| s.boot_time).min();
    let is_open = day.sessions.iter().any(|s| s.shutdown_time.is_none());

    let start_str = earliest
        .map(|t| t.format("%H:%M").to_string())
        .unwrap_or_else(|| "00:00".to_string());

    let end_str = if is_open {
        "now".to_string()
    } else {
        day.sessions
            .iter()
            .filter_map(|s| s.shutdown_time)
            .max()
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| "24:00".to_string())
    };

    format!("{start_str} → {end_str}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eventlog::Segment;
    use chrono::{DateTime, Local, NaiveDate, TimeZone};

    fn make_time(hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 10, 14, hour, minute, 0)
            .single()
            .expect("valid test time")
    }

    #[test]
    fn empty_day_produces_all_off_cells() {
        let day = DaySummary {
            date: NaiveDate::from_ymd_opt(2026, 10, 14).expect("valid date"),
            ..Default::default()
        };
        let s = render_timeline_string(&day, 24);
        assert_eq!(s, "························");
    }

    #[test]
    fn active_segment_renders_blocks() {
        let day = DaySummary {
            date: NaiveDate::from_ymd_opt(2026, 10, 14).expect("valid date"),
            segments: vec![Segment {
                start: make_time(8, 0),
                end: make_time(12, 0),
                kind: SegmentKind::Active,
            }],
            ..Default::default()
        };
        let s = render_timeline_string(&day, 24);
        // 0..8 off (8 chars), 8..12 active (4 chars), 12..24 off (12 chars)
        assert_eq!(s, "········████············");
    }

    #[test]
    fn sleep_segment_renders_hatched_blocks() {
        let day = DaySummary {
            date: NaiveDate::from_ymd_opt(2026, 10, 14).expect("valid date"),
            segments: vec![
                Segment {
                    start: make_time(8, 0),
                    end: make_time(10, 0),
                    kind: SegmentKind::Active,
                },
                Segment {
                    start: make_time(10, 0),
                    end: make_time(12, 0),
                    kind: SegmentKind::Sleep,
                },
                Segment {
                    start: make_time(12, 0),
                    end: make_time(14, 0),
                    kind: SegmentKind::Active,
                },
            ],
            ..Default::default()
        };
        let s = render_timeline_string(&day, 24);
        assert_eq!(s, "········██░░██··········");
    }

    #[test]
    fn hibernate_takes_priority_over_sleep() {
        let day = DaySummary {
            date: NaiveDate::from_ymd_opt(2026, 10, 14).expect("valid date"),
            segments: vec![Segment {
                start: make_time(10, 0),
                end: make_time(12, 0),
                kind: SegmentKind::Hibernate,
            }],
            ..Default::default()
        };
        let s = render_timeline_string(&day, 24);
        assert_eq!(s, "··········▓▓············");
    }

    #[test]
    fn format_duration_works_correctly() {
        assert_eq!(format_duration(0), "0h 00m");
        assert_eq!(format_duration(3600), "1h 00m");
        assert_eq!(format_duration(3665), "1h 01m");
        assert_eq!(format_duration(36000), "10h 00m");
    }
}
