use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::{self, stdin, stdout, IsTerminal},
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::Parser;
use flerp::app::App;
use flerp::app_structs::Cli;
use flerp::headless::{self, HeadlessRequest};
use flerp::media::{GraphicsMode, MediaRenderer};
use flerp::ui_components::ui;

pub fn run_tui(cli: &Cli, graphics: GraphicsMode) -> Result<(), Box<dyn Error>> {
    if !stdin().is_terminal() || !stdout().is_terminal() {
        return Err("flerp requires an interactive terminal session".into());
    }

    // Probing the terminal for its graphics protocol writes an escape sequence
    // and reads the reply, so it has to happen while stdout is still the plain
    // screen and before raw mode swallows the response.
    let mut media = MediaRenderer::with_mode(graphics);

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_instance = App::new();

    // Dosya yükleme
    if let Some(path) = cli.file.as_deref() {
        if let Err(e) = app_instance.load_file(path) {
            eprintln!("Error loading file: {}", e);
            // Consider how to handle this error: maybe exit or continue without a file
        }
    }

    // Flags that place the session somewhere other than the top of the file.
    if let Some(query) = cli.search.as_deref() {
        app_instance.set_search_query(query);
    }
    if let Some(page) = cli.page {
        app_instance.goto_page(page);
    }

    let tick_rate = Duration::from_millis(250);
    let mut last_tick_poll = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app_instance.state, &mut media))?;

        let timeout = tick_rate
            .checked_sub(last_tick_poll.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? && !dispatch(event::read()?, &mut app_instance) {
            break;
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

/// Route one terminal event into the app. Returns false when it asked to quit.
fn dispatch(event: Event, app: &mut App) -> bool {
    match event {
        // Key releases and repeats arrive on some platforms; acting on them
        // would double every keystroke.
        Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key.code),
        _ => true,
    }
}

fn run_headless(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let Some(file) = cli.file.clone() else {
        return Err("--json, --text and --extract-images each need a file path".into());
    };

    let request = HeadlessRequest {
        file,
        json: cli.json,
        text: cli.text,
        extract_images: cli.extract_images.clone(),
        search: cli.search.clone(),
        search_options: cli.search_options(),
        keyword_limit: cli.keywords,
    };

    let stdout = stdout();
    headless::run(&request, &mut stdout.lock())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let outcome = if cli.is_headless() {
        run_headless(&cli)
    } else {
        run_tui(&cli, cli.graphics)
    };

    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        // `flerp --text big.pdf | head` closes the pipe early. That is the
        // reader saying it has enough, not a failure worth a message.
        Err(error) if is_broken_pipe(error.as_ref()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("flerp: {error}");
            ExitCode::FAILURE
        }
    }
}

fn is_broken_pipe(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
}
