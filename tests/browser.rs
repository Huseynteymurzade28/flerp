//! The directory tree pane: what it lists, how it folds, and what happens when
//! a file is picked out of it.
//!
//! The pane is lazy by design, so most of these assert on what it did *not*
//! read as much as on what it did.

use std::fs;
use std::path::PathBuf;

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use flerp::app::App;
use flerp::app_structs::{Cli, TAB_VIEWER};
use flerp::media::MediaRenderer;
use flerp::ui_components::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use clap::Parser;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 40;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "flerp-browser-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create fixture directory");
        Self { path }
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(&path, contents).expect("write fixture file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// A directory with a nested folder, a dotfile and a binary, so the tests can
/// tell "not listed" apart from "not there".
fn corpus(name: &str) -> TempDir {
    let directory = TempDir::new(name);
    directory.write("alpha.txt", "needle in alpha\nplain line\n");
    directory.write("readme.md", "needle needle in readme\n");
    directory.write(".hidden.txt", "needle in a dotfile\n");
    directory.write("nested/deep.txt", "needle down here\n");
    directory.write("blob.bin", "before\0after needle\n");
    directory
}

fn app_on(directory: &TempDir) -> App {
    let mut app = App::new();
    app.open_directory(&directory.path).expect("open the tree");
    app
}

fn draw(app: &mut App) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    let mut media = MediaRenderer::halfblocks();
    terminal
        .draw(|frame| ui(frame, &mut app.state, &mut media))
        .unwrap();
    terminal
}

fn screen(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .chunks(WIDTH as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// Type a query into the search box and commit it, as a user would.
fn search_for(app: &mut App, query: &str) {
    app.handle_key(KeyCode::Char('/'));
    for character in query.chars() {
        app.handle_key(KeyCode::Char(character));
    }
    app.handle_key(KeyCode::Enter);
}

fn names(app: &App) -> Vec<String> {
    app.state
        .browser
        .as_ref()
        .expect("a tree")
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect()
}

#[test]
fn the_tree_lists_the_top_level_with_folders_first_and_dotfiles_left_out() {
    let directory = corpus("top-level");
    let app = app_on(&directory);

    assert_eq!(
        names(&app),
        vec!["nested", "alpha.txt", "blob.bin", "readme.md"]
    );
}

#[test]
fn a_folder_stays_unread_until_it_is_opened() {
    let directory = corpus("lazy");
    let mut app = app_on(&directory);

    // `nested` is the first row, and its contents are not listed under it.
    assert!(!names(&app).contains(&"deep.txt".to_string()));

    app.handle_key(KeyCode::Enter);
    assert_eq!(
        names(&app),
        vec!["nested", "deep.txt", "alpha.txt", "blob.bin", "readme.md"]
    );

    // Closing it puts the tree back exactly as it was.
    app.handle_key(KeyCode::Enter);
    assert!(!names(&app).contains(&"deep.txt".to_string()));
}

#[test]
fn opening_a_folder_leaves_the_cursor_on_the_folder_rather_than_on_a_row_index() {
    let directory = corpus("anchor");
    let mut app = app_on(&directory);

    // Park the cursor on the last row, then open the folder above it. The rows
    // below the folder all shift down; the cursor must follow its file.
    app.handle_key(KeyCode::End);
    assert_eq!(selected(&app), "readme.md");

    app.handle_key(KeyCode::Up);
    app.handle_key(KeyCode::Up);
    app.handle_key(KeyCode::Up);
    assert_eq!(selected(&app), "nested");

    app.handle_key(KeyCode::Char('l'));
    assert_eq!(selected(&app), "nested");
    app.handle_key(KeyCode::End);
    assert_eq!(selected(&app), "readme.md");
}

fn selected(app: &App) -> String {
    app.state
        .browser
        .as_ref()
        .expect("a tree")
        .selected_entry()
        .expect("a selected row")
        .name
        .clone()
}

#[test]
fn picking_a_file_loads_it_and_hands_the_keyboard_to_the_pane_showing_it() {
    let directory = corpus("pick");
    let mut app = app_on(&directory);

    // Down to alpha.txt, which is the row after the folder.
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(selected(&app), "alpha.txt");

    app.handle_key(KeyCode::Enter);

    assert!(app.state.file_name.ends_with("alpha.txt"));
    assert!(app.state.file_content.contains("needle in alpha"));
    assert_eq!(app.state.current_tab, TAB_VIEWER);
    assert!(
        !app.state.browser_focused(),
        "the tree should let go once a file is chosen"
    );

    // And the pane is still there to choose the next one from.
    assert!(app.state.browser.is_some());
}

#[test]
fn ctrl_w_moves_the_keyboard_between_the_tree_and_the_pane() {
    let directory = corpus("focus");
    let mut app = app_on(&directory);

    assert!(app.state.browser_focused(), "a directory opens on the tree");

    app.handle_key_event(ctrl('w'));
    assert!(!app.state.browser_focused());

    // With the tree let go, j/k are the viewer's again.
    app.state.file_content = (1..=100)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.state.content_scroll, 1);

    app.handle_key_event(ctrl('w'));
    assert!(app.state.browser_focused());
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(
        app.state.content_scroll, 1,
        "the viewer should not move while the tree has the keyboard"
    );
}

#[test]
fn a_committed_search_counts_matches_across_the_whole_tree() {
    let directory = corpus("sweep");
    let mut app = app_on(&directory);

    search_for(&mut app, "needle");

    let browser = app.state.browser.as_ref().expect("a tree");
    let results = browser.results.as_ref().expect("a swept tree");

    // The dotfile and the binary are not searchable, and the nested file is,
    // whether or not its folder happens to be open on screen.
    assert_eq!(results.files_searched, 3);
    assert_eq!(results.match_count(), 4);

    assert_eq!(
        browser.match_count(&directory.path.join("alpha.txt"), false),
        Some(1)
    );
    assert_eq!(
        browser.match_count(&directory.path.join("readme.md"), false),
        Some(2)
    );
    assert_eq!(
        browser.match_count(&directory.path.join("nested/deep.txt"), false),
        Some(1)
    );
    assert_eq!(
        browser.match_count(&directory.path.join(".hidden.txt"), false),
        None
    );
}

#[test]
fn a_closed_folder_still_reports_the_matches_hiding_inside_it() {
    let directory = TempDir::new("folder-totals");
    directory.write("nested/one.txt", "needle needle\n");
    directory.write("nested/two.txt", "needle\n");
    directory.write("nested/quiet.txt", "nothing to see\n");
    directory.write("empty/nothing.txt", "nothing to see\n");
    let mut app = app_on(&directory);

    search_for(&mut app, "needle");
    let browser = app.state.browser.as_ref().expect("a tree");

    // `nested` is still closed: its total is the only way to know it is worth
    // opening at all.
    assert!(!browser.is_expanded(&directory.path.join("nested")));
    assert_eq!(
        browser.match_count(&directory.path.join("nested"), true),
        Some(3)
    );
    assert_eq!(
        browser.match_count(&directory.path.join("empty"), true),
        None
    );
}

#[test]
fn the_counts_follow_the_search_toggles() {
    let directory = TempDir::new("toggles");
    directory.write("cased.txt", "Needle\nneedle\n");
    let mut app = app_on(&directory);

    app.state.case_sensitive = true;
    search_for(&mut app, "needle");
    let path = directory.path.join("cased.txt");
    assert_eq!(
        app.state
            .browser
            .as_ref()
            .unwrap()
            .match_count(&path, false),
        Some(1)
    );

    // `c` flips case sensitivity, and the tree's counts have to follow it or
    // they describe a search nobody asked for.
    app.handle_key_event(ctrl('w'));
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(
        app.state
            .browser
            .as_ref()
            .unwrap()
            .match_count(&path, false),
        Some(2)
    );
}

#[test]
fn backing_out_of_a_search_clears_the_counts() {
    let directory = corpus("clear");
    let mut app = app_on(&directory);

    search_for(&mut app, "needle");
    assert!(app.state.browser.as_ref().unwrap().results.is_some());

    app.handle_key(KeyCode::Char('/'));
    for _ in 0.."needle".len() {
        app.handle_key(KeyCode::Backspace);
    }
    app.handle_key(KeyCode::Enter);

    assert!(
        app.state.browser.as_ref().unwrap().results.is_none(),
        "an empty query should leave the tree as it was found"
    );
}

#[test]
fn picking_a_file_out_of_a_swept_tree_lands_on_its_first_match() {
    let directory = TempDir::new("land");
    directory.write(
        "long.txt",
        &format!("{}needle here\n", "filler\n".repeat(60)),
    );
    let mut app = app_on(&directory);

    search_for(&mut app, "needle");
    app.handle_key(KeyCode::Enter);

    assert!(app.state.file_name.ends_with("long.txt"));
    assert_eq!(app.state.current_tab, TAB_VIEWER);
    assert!(
        app.state.content_scroll > 0,
        "the viewer should be sitting on the match, not at the top"
    );
}

#[test]
fn the_tree_draws_beside_every_mode_and_shows_its_counts() {
    let directory = corpus("render");
    let mut app = app_on(&directory);
    search_for(&mut app, "needle");

    for tab in 0..6 {
        app.state.current_tab = tab;
        let terminal = draw(&mut app);
        let screen = screen(&terminal);

        assert!(screen.contains("Files"), "mode {tab} lost the tree pane");
        assert!(screen.contains("alpha.txt"), "mode {tab} lost the entries");
        assert!(screen.contains("×2"), "mode {tab} lost the match counts");
    }
}

#[test]
fn clicking_the_tree_reaches_for_it_and_a_second_click_opens_the_file() {
    let directory = corpus("click");
    let mut app = app_on(&directory);
    app.handle_key_event(ctrl('w'));
    assert!(!app.state.browser_focused());

    let terminal = draw(&mut app);
    let rows = app.state.hit.browser;

    // Row 1 is alpha.txt: the folder is row 0.
    app.handle_mouse(click(rows.x + 4, rows.y + 1));
    assert!(app.state.browser_focused(), "a click reaches for the pane");
    assert_eq!(selected(&app), "alpha.txt");
    assert!(
        app.state.file_content.is_empty(),
        "the first click picks the row without opening it"
    );

    drop(terminal);
    app.handle_mouse(click(rows.x + 4, rows.y + 1));
    assert!(app.state.file_name.ends_with("alpha.txt"));
}

#[test]
fn a_bare_directory_on_the_command_line_opens_the_tui() {
    let directory = corpus("cli");
    let path = directory.path.display().to_string();

    let cli = Cli::parse_from(["flerp", &path]);
    assert!(!cli.is_headless());
    assert_eq!(cli.browser_root(), Some(directory.path.clone()));

    // A query alongside it is still a grep, because piping that somewhere is
    // the whole point of asking for it on the command line.
    let searched = Cli::parse_from(["flerp", &path, "--search", "needle"]);
    assert!(searched.is_headless());

    // And a file is still a file.
    let file = directory.path.join("alpha.txt").display().to_string();
    let single = Cli::parse_from(["flerp", &file]);
    assert!(!single.is_headless());
    assert_eq!(single.browser_root(), None);
}

#[test]
fn flerp_on_its_own_opens_the_working_directory() {
    let bare = Cli::parse_from(["flerp"]);
    assert!(!bare.is_headless());
    assert_eq!(bare.browser_root(), Some(PathBuf::from(".")));

    // A query with no path is somebody at a prompt, not a pipeline: it opens
    // the tree with the search already run rather than printing.
    let searched = Cli::parse_from(["flerp", "--search", "needle"]);
    assert!(!searched.is_headless());
    assert_eq!(searched.browser_root(), Some(PathBuf::from(".")));

    // The output flags still need something to describe, and say so.
    assert!(Cli::parse_from(["flerp", "--json"]).is_headless());
}

#[test]
fn f_moves_the_keyboard_because_ctrl_w_closes_the_tab_in_some_terminals() {
    let directory = corpus("f-key");
    let mut app = app_on(&directory);
    assert!(app.state.browser_focused());

    app.handle_key(KeyCode::Char('f'));
    assert!(!app.state.browser_focused());

    app.handle_key(KeyCode::Char('f'));
    assert!(
        app.state.browser_focused(),
        "the same key has to bring the tree back"
    );

    // And it survives being pressed from inside the pane it is leaving.
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(selected(&app), "alpha.txt");
    app.handle_key(KeyCode::Char('f'));
    assert!(!app.state.browser_focused());
    assert_eq!(
        selected(&app),
        "alpha.txt",
        "leaving the pane should not move its cursor"
    );
}

#[test]
fn the_tree_marks_the_loaded_file_and_says_how_to_get_back_to_it() {
    let directory = corpus("marker");
    let mut app = app_on(&directory);

    let before = screen(&draw(&mut app));
    assert!(
        !before.contains('●'),
        "nothing is loaded yet, so nothing should be marked"
    );

    // Down to alpha.txt and open it.
    app.handle_key(KeyCode::Char('j'));
    app.handle_key(KeyCode::Enter);

    let after = screen(&draw(&mut app));
    assert!(
        after.contains("● alpha.txt"),
        "the loaded file should be marked in the tree"
    );
    // With the keyboard gone from the pane, the way back has to be on screen.
    assert!(after.contains("Files · f"), "screen was:\n{after}");
    assert!(after.contains("f back to files"), "screen was:\n{after}");
}

#[test]
fn a_tree_with_no_query_says_nothing_about_matches() {
    let directory = corpus("quiet");
    let mut app = app_on(&directory);

    let terminal = draw(&mut app);
    assert!(
        !screen(&terminal).contains('×'),
        "the pane should not invent counts before a search has run"
    );
}
