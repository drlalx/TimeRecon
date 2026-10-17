#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
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

#[cfg(windows)]
fn setup_console_for_cli() {
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
        STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };

    unsafe {
        // Try to attach to the parent console (e.g. cmd, powershell).
        // If not attached to any console (e.g. invoked via desktop shortcut with --tui), allocate a new one.
        let attached = AttachConsole(ATTACH_PARENT_PROCESS).is_ok();
        if !attached {
            let _ = AllocConsole();
        }

        // When a Windows subsystem application attaches to a console, STD_INPUT_HANDLE,
        // STD_OUTPUT_HANDLE, and STD_ERROR_HANDLE might be NULL or invalid.
        // Re-open and set standard console handles so crossterm and std::io can read and write.
        let std_in = GetStdHandle(STD_INPUT_HANDLE);
        if std_in.as_ref().is_err_and(|_| true) || std_in.as_ref().is_ok_and(|h| h.is_invalid()) {
            use windows::Win32::Storage::FileSystem::{
                CreateFileW, FILE_GENERIC_READ, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            };
            use windows::core::PCWSTR;
            let conin: Vec<u16> = "CONIN$\0".encode_utf16().collect();
            if let Ok(handle) = CreateFileW(
                PCWSTR(conin.as_ptr()),
                FILE_GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            ) {
                let _ = SetStdHandle(STD_INPUT_HANDLE, handle);
            }
        }

        let std_out = GetStdHandle(STD_OUTPUT_HANDLE);
        if std_out.as_ref().is_err_and(|_| true) || std_out.as_ref().is_ok_and(|h| h.is_invalid()) {
            use windows::Win32::Storage::FileSystem::{
                CreateFileW, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            };
            use windows::core::PCWSTR;
            let conout: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
            if let Ok(handle) = CreateFileW(
                PCWSTR(conout.as_ptr()),
                FILE_GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            ) {
                let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
                let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
            }
        }
    }
}

fn main() {
    let args_list: Vec<String> = std::env::args().skip(1).collect();
    let mut requested_gui = false;
    let mut requested_tui = false;
    let mut iter = args_list.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                #[cfg(windows)]
                setup_console_for_cli();
                print_usage();
                return;
            }
            "-v" | "--version" => {
                #[cfg(windows)]
                setup_console_for_cli();
                println!("TimeRecon 0.1.0");
                return;
            }
            "-s" | "--status" => {
                #[cfg(windows)]
                setup_console_for_cli();
                print_status();
                return;
            }
            "-e" | "--export" => {
                #[cfg(windows)]
                setup_console_for_cli();
                let target_path = iter.next().map(PathBuf::from);
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
                #[cfg(windows)]
                setup_console_for_cli();
                eprintln!("Unrecognized option: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
    }

    if requested_tui {
        #[cfg(feature = "tui")]
        {
            #[cfg(windows)]
            setup_console_for_cli();
            if let Err(err) = tui::run() {
                eprintln!("TUI error: {err}");
                std::process::exit(1);
            }
            return;
        }
        #[cfg(not(feature = "tui"))]
        {
            #[cfg(windows)]
            setup_console_for_cli();
            eprintln!("Error: TimeRecon was compiled without the 'tui' feature.");
            std::process::exit(1);
        }
    }

    if requested_gui {
        #[cfg(feature = "egui-ui")]
        {
            if let Err(err) = egui_ui::run() {
                #[cfg(windows)]
                setup_console_for_cli();
                eprintln!("GUI error: {err}");
                std::process::exit(1);
            }
            return;
        }
        #[cfg(not(feature = "egui-ui"))]
        {
            #[cfg(windows)]
            setup_console_for_cli();
            eprintln!("Error: TimeRecon was compiled without the 'egui-ui' feature.");
            std::process::exit(1);
        }
    }

    // Default mode resolution: GUI if display is present, else TUI
    #[cfg(feature = "egui-ui")]
    if is_display_available() {
        if let Err(err) = egui_ui::run() {
            #[cfg(windows)]
            setup_console_for_cli();
            eprintln!("GUI error: {err}");
            std::process::exit(1);
        }
        return;
    }

    #[cfg(feature = "tui")]
    {
        #[cfg(windows)]
        setup_console_for_cli();
        if let Err(err) = tui::run() {
            eprintln!("TUI error: {err}");
            std::process::exit(1);
        }
        return;
    }

    #[allow(unreachable_code)]
    {
        #[cfg(windows)]
        setup_console_for_cli();
        eprintln!("No suitable UI backend available. Enable 'egui-ui' or 'tui'.");
        std::process::exit(1);
    }
}
