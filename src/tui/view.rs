//! Terminal UI rendering and view components using Ratatui.

use crate::tui::app::{Tab, TuiApp};
use crate::tui::timeline::{format_day_span, format_duration, render_timeline_cells, CellKind};
use chrono::Local;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

/// Draws the complete TUI interface on the frame.
pub fn draw(frame: &mut Frame, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(8),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_tabs(frame, chunks[1], app);

    match app.selected_tab {
        Tab::Overview => render_overview_tab(frame, chunks[2], app),
        Tab::Sessions => render_sessions_tab(frame, chunks[2], app),
    }

    render_footer(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let pc_status_span = if app.is_pc_on() {
        Span::styled("● PC on", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("● PC off", Style::default().fg(Color::DarkGray))
    };

    let today_work = format_duration(app.today_work_seconds());
    let today_sleep = format_duration(app.today_sleep_seconds());
    let (month_total, month_avg, month_count) = app.month_stats();

    let lines = vec![
        Line::from(vec![
            Span::styled("TimeRecon", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" v0.1.0  —  Passive Uptime & Activity Tracker             "),
            pc_status_span,
        ]),
        Line::from(vec![
            Span::raw("Today: "),
            Span::styled(today_work, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" (Active) | Sleep: "),
            Span::styled(today_sleep, Style::default().fg(Color::Yellow)),
            Span::raw("  •  Month: "),
            Span::styled(format_duration(month_total), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" | Avg: "),
            Span::styled(format_duration(month_avg), Style::default().fg(Color::White)),
            Span::raw(format!(" ({month_count} active days)")),
        ]),
    ];

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 75, 95)));

    let paragraph = Paragraph::new(lines).block(header_block);
    frame.render_widget(paragraph, area);
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let titles = [Tab::Overview.title(), Tab::Sessions.title()];
    let selected_index = match app.selected_tab {
        Tab::Overview => 0,
        Tab::Sessions => 1,
    };

    let tabs = Tabs::new(titles.iter().copied())
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::Rgb(60, 75, 95))))
        .select(selected_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

fn render_overview_tab(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let today = Local::now().date_naive();
    // Timeline width depends on remaining screen width, at least 24
    let timeline_width = (area.width.saturating_sub(48)).clamp(24, 72) as usize;

    let header_cells = [
        "Date",
        "Work",
        "Sleep",
        "Span",
        "24h Timeline (00h ... 12h ... 24h)",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .summaries
        .iter()
        .enumerate()
        .map(|(index, summary)| {
            let is_today = summary.date == today;
            let date_label = if is_today {
                "Today".to_string()
            } else {
                let weekday_str = summary.date.format("%a").to_string();
                let weekday = match weekday_str.as_str() {
                    "Mon" => "Mo",
                    "Tue" => "Di",
                    "Wed" => "Mi",
                    "Thu" => "Do",
                    "Fri" => "Fr",
                    "Sat" => "Sa",
                    "Sun" => "So",
                    other => other,
                };
                format!("{weekday} {}", summary.date.format("%d.%m"))
            };

            let work_str = format_duration(summary.total_seconds);
            let sleep_str = format_duration(summary.sleep_seconds);
            let span_str = format_day_span(summary);

            let timeline_cells = render_timeline_cells(summary, timeline_width);
            let spans: Vec<Span> = timeline_cells
                .into_iter()
                .map(|cell| match cell {
                    CellKind::Active => Span::styled("█", Style::default().fg(Color::Cyan)),
                    CellKind::Sleep => Span::styled("░", Style::default().fg(Color::Yellow)),
                    CellKind::Hibernate => Span::styled("▓", Style::default().fg(Color::Magenta)),
                    CellKind::Off => Span::styled("·", Style::default().fg(Color::DarkGray)),
                })
                .collect();

            let row_cells = vec![
                Cell::from(date_label).style(if is_today {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
                Cell::from(work_str).style(Style::default().fg(Color::Cyan)),
                Cell::from(sleep_str).style(Style::default().fg(Color::Yellow)),
                Cell::from(span_str).style(Style::default().fg(Color::Gray)),
                Cell::from(Line::from(spans)),
            ];

            let row = Row::new(row_cells);
            if index == app.selected_day_index {
                row.style(Style::default().bg(Color::Rgb(24, 32, 45)).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(10), // Date
        Constraint::Length(9),  // Work
        Constraint::Length(9),  // Sleep
        Constraint::Length(15), // Span
        Constraint::Min(24),    // Timeline
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Daily Overview (last 120 days) ").border_style(Style::default().fg(Color::Rgb(60, 75, 95))));

    frame.render_widget(table, area);
}

fn render_sessions_tab(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let header_cells = ["Session", "Date", "Boot Time", "Shutdown Time", "Duration", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let is_running = session.shutdown_time.is_none();
            let date_str = session.boot_time.format("%Y-%m-%d").to_string();
            let start_str = session.boot_time.format("%H:%M:%S").to_string();
            let end_str = session
                .shutdown_time
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "now (running)".to_string());
            let duration_str = format_duration(session.duration_secs());

            let status_cell = if is_running {
                Cell::from("Active").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Cell::from("Finished").style(Style::default().fg(Color::DarkGray))
            };

            let row_cells = vec![
                Cell::from(format!("#{}", index + 1)).style(Style::default().fg(Color::DarkGray)),
                Cell::from(date_str).style(Style::default().fg(Color::White)),
                Cell::from(start_str).style(Style::default().fg(Color::Cyan)),
                Cell::from(end_str).style(Style::default().fg(Color::Gray)),
                Cell::from(duration_str).style(Style::default().fg(Color::Cyan)),
                status_cell,
            ];

            Row::new(row_cells)
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(16),
        Constraint::Length(10),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Recorded PC Sessions ").border_style(Style::default().fg(Color::Rgb(60, 75, 95))));

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 75, 95)));

    let content = if let Some(msg) = &app.status_message {
        Line::from(vec![
            Span::styled(msg, Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Quit  "),
            Span::styled("[Tab]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Tabs  "),
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Select Day  "),
            Span::styled("[r]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Refresh  "),
            Span::styled("[e]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Export CSV    "),
            Span::styled("█", Style::default().fg(Color::Cyan)),
            Span::raw(" Active  "),
            Span::styled("░", Style::default().fg(Color::Yellow)),
            Span::raw(" Sleep  "),
            Span::styled("▓", Style::default().fg(Color::Magenta)),
            Span::raw(" Hibernate  "),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            Span::raw(" Off"),
        ])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}
