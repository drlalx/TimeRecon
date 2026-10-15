//! Application composition and UI entry point.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod egui_ui;
mod eventlog;

fn main() -> eframe::Result {
    egui_ui::run()
}
