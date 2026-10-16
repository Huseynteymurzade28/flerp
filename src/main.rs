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
use flerp::editor;
use flerp::headless::{self, HeadlessRequest};
use flerp::media::{GraphicsMode, MediaRenderer};
use flerp::ui_components::ui;

pub fn run_tui(cli: &Cli, graphics: GraphicsMode) -> Result<(), Box<dyn Error>> {
    if !stdin().is_terminal() || !stdout().is_terminal() {
        // The `flerp: ` prefix is added when the error is printed, so messages
        // start with what went wrong rather than repeating the program's name.
        return Err(
            "an interactive terminal is required; --json and --text work without one".into(),
        );
    }

    // Probing the terminal for its graphics protocol writes an escape sequence
    // and reads the reply, so it has to happen while stdout is still the plain
    // screen and before raw mode swallows the response.
    let mut media = MediaRenderer::with_mode(graphics);

    let mut app_instance = App::new();

    // Everything the command line asks for is done before the alternate screen
    // is entered, so a bad path is reported on the ordinary terminal rather than
    // flashed onto a screen that is about to be torn down.
    //
    // A directory becomes the tree pane and loads nothing; anything else is the
    // single file the session opens on.
    match cli.browser_root() {
        Some(root) => app_instance.open_directory(&root)?,
        None => {
            if let Some(path) = cli.paths.first() {
                app_instance.load_file(path)?;
            }
        }
    }

    // Flags that place the session somewhere other than the top of the file.
    if let Some(query) = cli.search.as_deref() {
        app_instance.set_search_query(query);
    }
    if let Some(page) = cli.page {
        app_instance.goto_page(page);
    }

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

        if let Some(request) = app_instance.take_edit_request() {
            // The editor needs the terminal as an ordinary screen: cooked
            // input, the main buffer, and no mouse reports arriving as text.
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            let outcome = editor::open(&request);

            enable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )?;
            // ratatui only draws what changed since the last frame, and the
            // editor has scribbled over all of it.
            terminal.clear()?;

            app_instance.finish_edit(&request, outcome);
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
        Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key_event(key),
        Event::Mouse(mouse) => app.handle_mouse(mouse),
        _ => true,
    }
}

fn run_headless(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let request = HeadlessRequest {
        targets: cli.paths.clone(),
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
