//! Terminal UI state management.

use crate::eventlog::{self, DaySummary, PcSession};
use chrono::Local;
use std::path::PathBuf;

const OVERVIEW_DAYS: i64 = 120;

/// Active tab in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Sessions,
}

impl Tab {
    pub const fn next(self) -> Self {
        match self {
            Self::Overview => Self::Sessions,
            Self::Sessions => Self::Overview,
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "1: Daily Overview",
            Self::Sessions => "2: Sessions",
        }
    }
}

/// State of the TUI application.
pub struct TuiApp {
    pub summaries: Vec<DaySummary>,
    pub sessions: Vec<PcSession>,
    pub selected_tab: Tab,
    pub selected_day_index: usize,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl TuiApp {
    /// Loads session and summary data from event logs.
    pub fn new() -> Self {
        let mut app = Self {
            summaries: Vec::new(),
            sessions: Vec::new(),
            selected_tab: Tab::Overview,
            selected_day_index: 0,
            status_message: None,
            should_quit: false,
        };
        app.refresh();
        app
    }

    /// Re-reads system logs and updates summaries and sessions.
    pub fn refresh(&mut self) {
        self.summaries = eventlog::daily_summaries(OVERVIEW_DAYS);
        self.sessions = eventlog::read_pc_sessions(OVERVIEW_DAYS);
        if self.selected_day_index >= self.summaries.len() && !self.summaries.is_empty() {
            self.selected_day_index = self.summaries.len().saturating_sub(1);
        }
    }

    /// Exports CSV data to the user's Documents directory.
    pub fn export_csv(&mut self) {
        let mut path = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        path.push(format!("timerecon-export_{timestamp}.csv"));

        match std::fs::write(&path, eventlog::export_csv(OVERVIEW_DAYS)) {
            Ok(()) => {
                self.status_message = Some(format!("✓ CSV saved: {}", path.display()));
            }
            Err(error) => {
                self.status_message = Some(format!("✗ CSV error: {error}"));
            }
        }
    }

    pub fn next_day(&mut self) {
        if !self.summaries.is_empty() {
            self.selected_day_index = (self.selected_day_index + 1).min(self.summaries.len().saturating_sub(1));
        }
    }

    pub fn prev_day(&mut self) {
        self.selected_day_index = self.selected_day_index.saturating_sub(1);
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn is_pc_on(&self) -> bool {
        self.sessions.first().is_some_and(|s| s.shutdown_time.is_none())
    }

    pub fn today_work_seconds(&self) -> i64 {
        self.summaries.first().map(|d| d.total_seconds).unwrap_or(0)
    }

    pub fn today_sleep_seconds(&self) -> i64 {
        self.summaries.first().map(|d| d.sleep_seconds).unwrap_or(0)
    }

    pub fn month_stats(&self) -> (i64, i64, usize) {
        let current_month = Local::now().format("%Y-%m").to_string();
        let month_days: Vec<&DaySummary> = self
            .summaries
            .iter()
            .filter(|d| d.date.format("%Y-%m").to_string() == current_month && d.total_seconds > 0)
            .collect();

        let total_secs: i64 = month_days.iter().map(|d| d.total_seconds).sum();
        let count = month_days.len();
        let avg_secs = if count > 0 {
            total_secs / (count as i64)
        } else {
            0
        };

        (total_secs, avg_secs, count)
    }
}
