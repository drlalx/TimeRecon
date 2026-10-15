use chrono::{DateTime, Datelike, Local, Timelike};
use eframe::egui::{
    self, Align, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, Rect,
    RichText, Sense, Shape, Stroke, Vec2,
};
use std::path::PathBuf;
use std::sync::Arc;

use crate::eventlog::{self, SegmentKind};

const OVERVIEW_DAYS: i64 = 120;

const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 46);
const SURFACE: Color32 = Color32::from_rgb(49, 50, 68);
const GRID: Color32 = Color32::from_rgb(69, 71, 90);
const TEXT: Color32 = Color32::from_rgb(205, 214, 244);
const MUTED: Color32 = Color32::from_rgb(166, 173, 200);
const FAINT: Color32 = Color32::from_rgb(108, 112, 134);
const BLUE: Color32 = Color32::from_rgb(137, 180, 250);
const GREEN: Color32 = Color32::from_rgb(166, 227, 161);
const RED: Color32 = Color32::from_rgb(243, 139, 168);
const PEACH: Color32 = Color32::from_rgb(250, 179, 135);
const PEACH_DARK: Color32 = Color32::from_rgb(201, 138, 100);
const MAUVE: Color32 = Color32::from_rgb(203, 166, 247);
const MAUVE_DARK: Color32 = Color32::from_rgb(159, 130, 196);

const DATE_COLUMN_WIDTH: f32 = 70.0;
const VALUE_COLUMN_WIDTH: f32 = 56.0;
const BAR_HEIGHT: f32 = 14.0;

pub fn run() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]);
    match eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/app-256.png")) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(error) => eprintln!("Failed to load window icon: {error}"),
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "TimeRecon",
        options,
        Box::new(|creation_context| Ok(Box::new(ActiveTimeApp::new(&creation_context.egui_ctx)?))),
    )
}

struct ActiveTimeApp {
    dashboard: Dashboard,
    selected_tab: Tab,
    status: Option<Status>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Sessions,
}

struct Status {
    message: String,
    export_path: Option<PathBuf>,
}

struct Dashboard {
    days: Vec<Day>,
    sessions: Vec<Session>,
    today_duration: String,
    current_session_start: String,
    pc_is_on: bool,
    month_total: String,
    month_average: String,
    month_sleep: String,
    work_days: usize,
}

#[derive(Default)]
struct Day {
    weekday: String,
    date: String,
    duration: String,
    sleep: String,
    span: String,
    start: String,
    end: String,
    bar_start: f32,
    bar_width: f32,
    segments: Vec<DaySegment>,
    is_today: bool,
}

#[derive(Default)]
struct DaySegment {
    start: f32,
    width: f32,
    kind: SegmentKind,
    tooltip: String,
}

struct Session {
    start: String,
    end: String,
    duration: String,
    is_active: bool,
}

impl ActiveTimeApp {
    fn new(context: &egui::Context) -> std::io::Result<Self> {
        configure_fonts(context);
        let selected_tab = if std::env::var("TIMERECON_TAB").as_deref() == Ok("sessions") {
            Tab::Sessions
        } else {
            Tab::Overview
        };
        Ok(Self {
            dashboard: Dashboard::load(),
            selected_tab,
            status: None,
        })
    }

    fn export_csv(&mut self) {
        let mut path = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        path.push(format!("timerecon-export_{timestamp}.csv"));

        match std::fs::write(&path, eventlog::export_csv(OVERVIEW_DAYS)) {
            Ok(()) => {
                self.status = Some(Status {
                    message: format!("CSV saved: {} (click to open)", path.display()),
                    export_path: Some(path),
                });
            }
            Err(error) => {
                self.status = Some(Status {
                    message: format!("CSV error: {error}"),
                    export_path: None,
                });
            }
        }
    }

    fn open_export(&mut self) {
        let Some(status) = &mut self.status else {
            return;
        };
        let Some(path) = &status.export_path else {
            return;
        };

        if let Err(error) = open_file(path) {
            status.message = format!("Could not open CSV: {error}");
            status.export_path = None;
        }
    }
}

impl eframe::App for ActiveTimeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = BACKGROUND;
        visuals.panel_fill = BACKGROUND;
        visuals.faint_bg_color = SURFACE;
        visuals.widgets.noninteractive.bg_fill = SURFACE;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        ui.ctx().set_visuals(visuals);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            self.show_header(ui);
            self.show_summary(ui);
            self.show_controls(ui);
            self.show_status(ui);
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.selected_tab {
                    Tab::Overview => self.show_overview(ui),
                    Tab::Sessions => self.show_sessions(ui),
                });
        });
    }
}

impl ActiveTimeApp {
    fn show_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("TimeRecon").color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Today: {}", self.dashboard.today_duration)).color(BLUE),
                );
                ui.label(
                    RichText::new(if self.dashboard.pc_is_on {
                        "● PC on"
                    } else {
                        "● PC off"
                    })
                    .color(if self.dashboard.pc_is_on { GREEN } else { RED }),
                );
            });
        });
    }

    fn show_summary(&self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(SURFACE)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "Current session since: {}",
                        self.dashboard.current_session_start
                    ))
                    .color(MUTED),
                );
            });
        egui::Frame::new()
            .fill(SURFACE)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Month: {}", self.dashboard.month_total)).color(TEXT),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Average: {}", self.dashboard.month_average))
                            .color(MUTED),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Sleep: {}", self.dashboard.month_sleep))
                            .color(PEACH),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{} work days", self.dashboard.work_days))
                            .color(FAINT),
                    );
                });
            });
    }

    fn show_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.selected_tab, Tab::Overview, "Daily overview");
            ui.selectable_value(&mut self.selected_tab, Tab::Sessions, "Sessions");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("CSV export").clicked() {
                    self.export_csv();
                }
                if ui.button("Refresh").clicked() {
                    self.dashboard = Dashboard::load();
                    self.status = None;
                }
            });
        });
    }

    fn show_status(&mut self, ui: &mut egui::Ui) {
        let Some(status) = &self.status else {
            return;
        };

        let response = ui.add(
            egui::Label::new(RichText::new(&status.message).color(GREEN)).sense(Sense::click()),
        );
        if response.clicked() && status.export_path.is_some() {
            self.open_export();
        }
    }

    fn show_overview(&self, ui: &mut egui::Ui) {
        show_legend(ui);
        show_axis(ui);
        for day in &self.dashboard.days {
            let fill = if day.is_today { SURFACE } else { BACKGROUND };
            egui::Frame::new()
                .fill(fill)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        fixed_cell(
                            ui,
                            DATE_COLUMN_WIDTH,
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.label(RichText::new(&day.weekday).color(FAINT));
                                ui.label(RichText::new(&day.date).color(if day.is_today {
                                    TEXT
                                } else {
                                    FAINT
                                }));
                            },
                        );

                        let (response, painter) =
                            ui.allocate_painter(Vec2::new(track_width(ui), 18.0), Sense::hover());
                        let track = response.rect;
                        paint_track(&painter, track);
                        paint_span(&painter, track, day.bar_start, day.bar_width);
                        for segment in &day.segments {
                            paint_segment(&painter, segment_rect(track, segment), segment.kind);
                        }

                        let tooltip = response
                            .hover_pos()
                            .and_then(|position| {
                                day.segments.iter().find(|segment| {
                                    segment_rect(track, segment).x_range().contains(position.x)
                                })
                            })
                            .map(|segment| segment.tooltip.clone())
                            .unwrap_or_else(|| format!("{} - {}", day.start, day.end));
                        response.on_hover_text(tooltip);

                        value_cell(ui, &day.duration, if day.is_today { GREEN } else { MUTED });
                        value_cell(ui, &day.sleep, PEACH);
                        value_cell(ui, &day.span, FAINT);
                    });
                });
        }
    }

    fn show_sessions(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Sessions today").color(TEXT).strong());
        for session in &self.dashboard.sessions {
            egui::Frame::new()
                .fill(SURFACE)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("▌").color(if session.is_active {
                            GREEN
                        } else {
                            FAINT
                        }));
                        ui.label(RichText::new(&session.start).color(BLUE));
                        ui.label(RichText::new("→").color(FAINT));
                        ui.label(RichText::new(&session.end).color(if session.is_active {
                            GREEN
                        } else {
                            BLUE
                        }));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(&session.duration).color(TEXT).strong());
                        });
                    });
                });
        }
    }
}

impl Dashboard {
    fn load() -> Self {
        let today = Local::now().date_naive();
        let summaries = eventlog::daily_summaries(OVERVIEW_DAYS);
        let sessions = eventlog::read_pc_sessions(OVERVIEW_DAYS);

        let days = summaries
            .iter()
            .map(|summary| {
                let (start, end, span, bar_start, bar_width) = summary
                    .sessions
                    .iter()
                    .map(|session| {
                        let start = session.boot_time;
                        let end = session.shutdown_time.unwrap_or_else(Local::now);
                        (start, end)
                    })
                    .reduce(|(earliest, latest), (start, end)| {
                        (earliest.min(start), latest.max(end))
                    })
                    .map(|(start, end)| {
                        (
                            start.format("%H:%M").to_string(),
                            end.format("%H:%M").to_string(),
                            format_duration((end - start).num_seconds().max(0)),
                            day_fraction(start),
                            (day_fraction(end) - day_fraction(start)).max(0.0),
                        )
                    })
                    .unwrap_or_else(|| ("-".to_owned(), "-".to_owned(), "-".to_owned(), 0.0, 0.0));
                let segments = summary
                    .segments
                    .iter()
                    .map(|segment| DaySegment {
                        start: day_fraction(segment.start),
                        width: (day_fraction(segment.end) - day_fraction(segment.start)).max(0.0),
                        kind: segment.kind,
                        tooltip: format!(
                            "{}\n{} - {} ({})",
                            segment_label(segment.kind),
                            segment.start.format("%H:%M"),
                            segment.end.format("%H:%M"),
                            format_duration(segment.duration_secs())
                        ),
                    })
                    .collect();

                Day {
                    weekday: weekday_german(summary.date.format("%a").to_string().as_str())
                        .to_owned(),
                    date: summary.date.format("%d.%m.").to_string(),
                    duration: format_duration(summary.total_seconds),
                    sleep: if summary.sleep_seconds >= 60 {
                        format_duration(summary.sleep_seconds)
                    } else {
                        "-".to_owned()
                    },
                    span,
                    start,
                    end,
                    bar_start,
                    bar_width,
                    segments,
                    is_today: summary.date == today,
                }
            })
            .collect();

        let today_duration = summaries
            .iter()
            .find(|summary| summary.date == today)
            .map(|summary| format_duration(summary.total_seconds))
            .unwrap_or_else(|| format_duration(0));
        let current_session_start = sessions
            .iter()
            .find(|session| session.shutdown_time.is_none())
            .map(|session| session.boot_time.format("%H:%M").to_string())
            .unwrap_or_else(|| "-".to_owned());
        let pc_is_on = sessions
            .iter()
            .any(|session| session.shutdown_time.is_none());
        let today_summary = summaries.iter().find(|summary| summary.date == today);
        let sessions = today_summary
            .map(|summary| summary.sessions.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|session| Session {
                start: session.boot_time.format("%H:%M").to_string(),
                end: session
                    .shutdown_time
                    .map(|time| time.format("%H:%M").to_string())
                    .unwrap_or_else(|| "now".to_owned()),
                duration: format_duration(session.duration_secs()),
                is_active: session.shutdown_time.is_none(),
            })
            .collect();

        let month_summaries: Vec<_> = summaries
            .iter()
            .filter(|summary| {
                summary.date.month() == today.month() && summary.date.year() == today.year()
            })
            .collect();
        let month_total_seconds: i64 = month_summaries
            .iter()
            .map(|summary| summary.total_seconds)
            .sum();
        let month_sleep_seconds: i64 = month_summaries
            .iter()
            .map(|summary| summary.sleep_seconds)
            .sum();
        let work_days = month_summaries
            .iter()
            .filter(|summary| summary.total_seconds > 0)
            .count();
        let month_average = if work_days == 0 {
            0
        } else {
            month_total_seconds / work_days as i64
        };

        Self {
            days,
            sessions,
            today_duration,
            current_session_start,
            pc_is_on,
            month_total: format_duration(month_total_seconds),
            month_average: format_duration(month_average),
            month_sleep: format_duration(month_sleep_seconds),
            work_days,
        }
    }
}

fn day_fraction(time: DateTime<Local>) -> f32 {
    time.num_seconds_from_midnight() as f32 / 86_400.0
}

fn segment_label(kind: SegmentKind) -> &'static str {
    match kind {
        SegmentKind::Active => "Active",
        SegmentKind::Sleep => "Sleep",
        SegmentKind::Hibernate => "Hibernate",
    }
}

fn track_width(ui: &egui::Ui) -> f32 {
    let values_width = 3.0 * (VALUE_COLUMN_WIDTH + ui.spacing().item_spacing.x);
    (ui.available_width() - values_width).max(0.0)
}

fn fixed_cell(ui: &mut egui::Ui, width: f32, layout: Layout, content: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(Vec2::new(width, 18.0), layout, |ui| {
        ui.set_min_width(width);
        content(ui);
    });
}

fn value_cell(ui: &mut egui::Ui, text: &str, color: Color32) {
    fixed_cell(
        ui,
        VALUE_COLUMN_WIDTH,
        Layout::right_to_left(Align::Center),
        |ui| {
            ui.label(RichText::new(text).color(color));
        },
    );
}

fn show_legend(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for kind in [
            SegmentKind::Active,
            SegmentKind::Sleep,
            SegmentKind::Hibernate,
        ] {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 10.0), Sense::hover());
            paint_segment(ui.painter(), rect, kind);
            ui.label(RichText::new(segment_label(kind)).color(MUTED));
            ui.add_space(8.0);
        }
        let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 10.0), Sense::hover());
        paint_dashed_rect(ui.painter(), rect);
        ui.label(RichText::new("Begin - end").color(MUTED));
    });
}

fn show_axis(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        fixed_cell(
            ui,
            DATE_COLUMN_WIDTH,
            Layout::left_to_right(Align::Center),
            |_| {},
        );
        let (response, painter) =
            ui.allocate_painter(Vec2::new(track_width(ui), 14.0), Sense::hover());
        let rect = response.rect;
        for hour in (0..=24).step_by(3) {
            let x = rect.left() + rect.width() * hour as f32 / 24.0;
            let anchor = match hour {
                0 => Align2::LEFT_TOP,
                24 => Align2::RIGHT_TOP,
                _ => Align2::CENTER_TOP,
            };
            painter.text(
                egui::pos2(x, rect.top()),
                anchor,
                hour.to_string(),
                FontId::proportional(11.0),
                FAINT,
            );
        }
        for header in ["Work", "Sleep", "Span"] {
            value_cell(ui, header, FAINT);
        }
    });
}

fn segment_rect(track: Rect, segment: &DaySegment) -> Rect {
    let top = track.center().y - BAR_HEIGHT / 2.0;
    Rect::from_min_size(
        egui::pos2(track.left() + track.width() * segment.start, top),
        Vec2::new((track.width() * segment.width).max(1.0), BAR_HEIGHT),
    )
}

fn paint_track(painter: &egui::Painter, track: Rect) {
    let top = track.center().y - BAR_HEIGHT / 2.0;
    for hour in 1..24 {
        let x = track.left() + track.width() * hour as f32 / 24.0;
        painter.line_segment(
            [egui::pos2(x, top), egui::pos2(x, top + BAR_HEIGHT)],
            Stroke::new(1.0_f32, GRID),
        );
    }
}

fn paint_span(painter: &egui::Painter, track: Rect, start: f32, width: f32) {
    if width <= 0.0 {
        return;
    }
    let rect = Rect::from_min_size(
        egui::pos2(track.left() + track.width() * start, track.top()),
        Vec2::new(track.width() * width, track.height()),
    );
    paint_dashed_rect(painter, rect);
}

fn paint_dashed_rect(painter: &egui::Painter, rect: Rect) {
    let points = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    painter.extend(Shape::dashed_line(
        &points,
        Stroke::new(1.0_f32, FAINT),
        4.0,
        3.0,
    ));
}

fn paint_segment(painter: &egui::Painter, rect: Rect, kind: SegmentKind) {
    let (fill, stripe) = match kind {
        SegmentKind::Active => {
            painter.rect_filled(rect, 2.0, BLUE);
            return;
        }
        SegmentKind::Sleep => (PEACH, PEACH_DARK),
        SegmentKind::Hibernate => (MAUVE, MAUVE_DARK),
    };
    painter.rect_filled(rect, 2.0, fill);
    let hatch = painter.with_clip_rect(rect);
    let mut x = rect.left() - rect.height();
    while x < rect.right() {
        hatch.line_segment(
            [
                egui::pos2(x, rect.bottom()),
                egui::pos2(x + rect.height(), rect.top()),
            ],
            Stroke::new(2.0_f32, stripe),
        );
        x += 6.0;
    }
}

fn weekday_german(weekday: &str) -> &str {
    match weekday {
        "Mon" => "Mo",
        "Tue" => "Di",
        "Wed" => "Mi",
        "Thu" => "Do",
        "Fri" => "Fr",
        "Sat" => "Sa",
        "Sun" => "So",
        other => other,
    }
}

fn configure_fonts(context: &egui::Context) {
    for &path_str in system_font_candidates() {
        if let Ok(font_bytes) = std::fs::read(path_str) {
            let mut fonts = FontDefinitions::empty();
            fonts.font_data.insert(
                "SystemFont".to_owned(),
                Arc::new(FontData::from_owned(font_bytes)),
            );

            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                if let Some(family_fonts) = fonts.families.get_mut(&family) {
                    family_fonts.push("SystemFont".to_owned());
                }
            }

            context.set_fonts(fonts);
            return;
        }
    }

    context.set_fonts(FontDefinitions::default());
}

fn system_font_candidates() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[r"C:\Windows\Fonts\segoeui.ttf"]
    }
    #[cfg(target_os = "macos")]
    {
        &["/System/Library/Fonts/SFPro.ttf", "/System/Library/Fonts/Helvetica.ttc"]
    }
    #[cfg(target_os = "linux")]
    {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        ]
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        &[]
    }
}

fn open_file(path: &std::path::Path) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    let opener = "explorer.exe";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(not(any(windows, target_os = "macos")))]
    let opener = "xdg-open";

    std::process::Command::new(opener).arg(path).spawn()
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    format!("{hours}h {minutes:02}m")
}

#[cfg(test)]
mod tests {
    use super::{format_duration, system_font_candidates};

    #[test]
    fn formats_duration_as_hours_and_minutes() {
        assert_eq!(format_duration(3_660), "1h 01m");
    }

    #[test]
    fn provides_system_font_candidates_for_target_platform() {
        let candidates = system_font_candidates();
        #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
        assert!(!candidates.is_empty());
    }
}
