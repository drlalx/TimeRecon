//! Application composition and entry point.

mod eventlog;

#[cfg(feature = "egui-ui")]
mod egui_ui;

#[cfg(feature = "tui")]
mod tui;

use std::path::PathBuf;

fn print_usage() {
    println!(
        "TimeRecon v0.1.0 - Reconstruct PC uptime and activity from system logs

Usage:
  time-recon [OPTIONS]

Options:
  -t, --tui             Launch Terminal UI (Ratatui)
  -g, --gui             Launch Desktop GUI (Egui)
  -s, --status          Print one-line status summary and exit
  -e, --export [PATH]   Export CSV data to file and exit
  -v, --version         Print version information
  -h, --help            Show this help message

Default behavior:
  Launches GUI if a display server is available; falls back to TUI in headless environments."
    );
}

fn print_status() {
    let summaries = eventlog::daily_summaries(120);
    let sessions = eventlog::read_pc_sessions(120);
    let is_on = sessions.first().is_some_and(|s| s.shutdown_time.is_none());
    let state = if is_on { "● PC on" } else { "● PC off" };
    let today_work = summaries.first().map(|d| d.total_seconds).unwrap_or(0);
    let today_sleep = summaries.first().map(|d| d.sleep_seconds).unwrap_or(0);
    let hours = today_work / 3600;
    let mins = (today_work % 3600) / 60;
    let sleep_h = today_sleep / 3600;
    let sleep_m = (today_sleep % 3600) / 60;
    println!("{state} | Today: {hours}h {mins:02}m (Active) | Sleep: {sleep_h}h {sleep_m:02}m");
}

fn export_csv(path: Option<PathBuf>) -> Result<(), std::io::Error> {
    let target = path.unwrap_or_else(|| {
        let mut p = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        p.push(format!("timerecon-export_{ts}.csv"));
        p
    });
    let csv = eventlog::export_csv(120);
    std::fs::write(&target, csv)?;
    println!("✓ CSV exported to: {}", target.display());
    Ok(())
}

fn is_display_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        true
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut requested_gui = false;
    let mut requested_tui = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "-v" | "--version" => {
                println!("TimeRecon 0.1.0");
                return;
            }
            "-s" | "--status" => {
                print_status();
                return;
            }
            "-e" | "--export" => {
                let target_path = args.next().map(PathBuf::from);
                if let Err(err) = export_csv(target_path) {
                    eprintln!("Failed to export CSV: {err}");
                    std::process::exit(1);
                }
                return;
            }
            "-g" | "--gui" => {
                requested_gui = true;
            }
            "-t" | "--tui" => {
                requested_tui = true;
            }
            other => {
                eprintln!("Unrecognized option: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
    }

    if requested_tui {
        #[cfg(feature = "tui")]
        {
            if let Err(err) = tui::run() {
                eprintln!("TUI error: {err}");
                std::process::exit(1);
            }
            return;
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("Error: TimeRecon was compiled without the 'tui' feature.");
            std::process::exit(1);
        }
    }

    if requested_gui {
        #[cfg(feature = "egui-ui")]
        {
            if let Err(err) = egui_ui::run() {
                eprintln!("GUI error: {err}");
                std::process::exit(1);
            }
            return;
        }
        #[cfg(not(feature = "egui-ui"))]
        {
            eprintln!("Error: TimeRecon was compiled without the 'egui-ui' feature.");
            std::process::exit(1);
        }
    }

    // Default mode resolution: GUI if display is present, else TUI
    #[cfg(feature = "egui-ui")]
    if is_display_available() {
        if let Err(err) = egui_ui::run() {
            eprintln!("GUI error: {err}");
            std::process::exit(1);
        }
        return;
    }

    #[cfg(feature = "tui")]
    {
        if let Err(err) = tui::run() {
            eprintln!("TUI error: {err}");
            std::process::exit(1);
        }
        return;
    }

    #[allow(unreachable_code)]
    {
        eprintln!("No suitable UI backend available. Enable 'egui-ui' or 'tui'.");
        std::process::exit(1);
    }
}
