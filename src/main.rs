use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::{stdin, stdout, IsTerminal},
    time::{Duration, Instant},
};

use clap::Parser;
use flerp::app::App;
use flerp::app_structs::Cli;
use flerp::media::MediaRenderer;
use flerp::ui_components::ui;

pub fn run_tui(file_path: Option<&str>) -> Result<(), Box<dyn Error>> {
    if !stdin().is_terminal() || !stdout().is_terminal() {
        return Err("flerp requires an interactive terminal session".into());
    }

    // Probing the terminal for its graphics protocol writes an escape sequence
    // and reads the reply, so it has to happen while stdout is still the plain
    // screen and before raw mode swallows the response.
    let mut media = MediaRenderer::detect();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_instance = App::new();

    // Dosya yükleme
    if let Some(path) = file_path {
        if let Err(e) = app_instance.load_file(path) {
            eprintln!("Error loading file: {}", e);
            // Consider how to handle this error: maybe exit or continue without a file
        }
    }

    let tick_rate = Duration::from_millis(250);
    let mut last_tick_poll = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app_instance.state, &mut media))?;

        let timeout = tick_rate
            .checked_sub(last_tick_poll.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // KeyCode is used by app_instance.handle_key, so it's not unused globally
                    if !app_instance.handle_key(key.code) {
                        break;
                    }
                }
            }
        }

        if last_tick_poll.elapsed() >= tick_rate {
            app_instance.tick();
            last_tick_poll = Instant::now();
        }
    }

    // Terminal temizleme
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse(); // clap::Parser is used here

    run_tui(cli.file.as_deref())
}
