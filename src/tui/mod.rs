//! Terminal user interface module powered by Ratatui and Crossterm.

pub mod app;
pub mod timeline;
pub mod view;

use app::{Tab, TuiApp};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout};
use std::time::Duration;

/// RAII Guard ensuring terminal settings are restored even on panic or error.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

/// Runs the interactive Terminal User Interface.
pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = TuiApp::new();

    loop {
        terminal.draw(|f| view::draw(f, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == event::KeyEventKind::Press
        {
            match (key.code, key.modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                            app.should_quit = true;
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        (KeyCode::Tab, _) => {
                            app.next_tab();
                        }
                        (KeyCode::Char('1'), _) => {
                            app.selected_tab = Tab::Overview;
                        }
                        (KeyCode::Char('2'), _) => {
                            app.selected_tab = Tab::Sessions;
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                            app.next_day();
                        }
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                            app.prev_day();
                        }
                        (KeyCode::Char('r'), _) | (KeyCode::F(5), _) => {
                            app.refresh();
                            app.status_message = Some("✓ Data refreshed from system logs".to_string());
                        }
                        (KeyCode::Char('e'), _) => {
                            app.export_csv();
                        }
                        _ => {}
                    }
            }
        }

    Ok(())
}
