//! The `e` key: which line it asks for, what it refuses, and how the session
//! picks back up afterwards. The editor itself is never launched here; the
//! event loop owns that, and these tests stand in for it.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::event::KeyCode;
use flerp::app::App;
use flerp::app_structs::{TAB_SEARCH, TAB_VIEWER};
use flerp::editor::{editor_argv, resolve_editor, EditRequest};

/// A scratch file under the target directory, unique to the test that asks.
fn scratch(name: &str, contents: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("editor");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, contents).unwrap();
    path
}

fn numbered(count: usize) -> String {
    (1..=count).map(|n| format!("line {n}\n")).collect()
}

fn app_on(path: &Path) -> App {
    let mut app = App::new();
    app.load_file(path.to_str().unwrap()).unwrap();
    app
}

/// An exit status that reports success, without depending on a real editor.
fn success() -> io::Result<std::process::ExitStatus> {
    Command::new("true").status()
}

#[test]
fn visual_wins_over_editor_and_vi_is_the_fallback() {
    let some = |s: &str| Some(s.to_string());
    assert_eq!(resolve_editor(some("hx"), some("nano")), "hx");
    assert_eq!(resolve_editor(None, some("nano")), "nano");
    assert_eq!(resolve_editor(some("  "), some("nano")), "nano");
    assert_eq!(resolve_editor(None, None), "vi");
}

#[test]
fn the_line_flag_goes_before_the_path_and_editor_arguments_survive() {
    let argv = editor_argv("emacs -nw", Path::new("notes.txt"), 12).unwrap();
    let expected: Vec<OsString> = ["emacs", "-nw", "+12", "notes.txt"]
        .into_iter()
        .map(OsString::from)
        .collect();
    assert_eq!(argv, expected);

    assert!(editor_argv("   ", Path::new("notes.txt"), 1).is_none());
}

#[test]
fn e_opens_on_the_top_line_of_the_viewer() {
    let path = scratch("viewer.txt", &numbered(100));
    let mut app = app_on(&path);
    app.state.current_tab = TAB_VIEWER;
    app.state.content_scroll = 41;

    app.handle_key(KeyCode::Char('e'));

    assert_eq!(
        app.take_edit_request(),
        Some(EditRequest { path, line: 42 })
    );
    assert_eq!(app.take_edit_request(), None, "a request is taken once");
}

#[test]
fn e_in_the_search_tab_opens_on_the_selected_match() {
    let path = scratch("search.txt", &numbered(100));
    let mut app = app_on(&path);
    app.set_search_query("line 7");
    assert_eq!(app.state.current_tab, TAB_SEARCH);
    // "line 7", "line 70" ... "line 79": step to the third match.
    app.handle_key(KeyCode::Down);
    app.handle_key(KeyCode::Down);

    app.handle_key(KeyCode::Char('e'));

    assert_eq!(app.take_edit_request().map(|r| r.line), Some(71));
}

#[test]
fn e_refuses_when_there_is_nothing_real_to_edit() {
    let mut app = App::new();
    app.state.file_content = "in memory only".to_string();
    app.handle_key(KeyCode::Char('e'));
    assert_eq!(app.take_edit_request(), None);
    assert!(app.state.status_message.contains("No file loaded"));
}

#[test]
fn e_refuses_files_whose_text_flerp_made_up() {
    let path = scratch("picture.png", "not really a png");
    let mut app = App::new();
    app.state.file_name = path.to_string_lossy().into_owned();

    app.handle_key(KeyCode::Char('e'));

    assert_eq!(app.take_edit_request(), None);
    assert!(app.state.status_message.contains("image"));
}

#[test]
fn coming_back_reloads_the_file_and_keeps_the_place() {
    let path = scratch("reload.txt", &numbered(100));
    let mut app = app_on(&path);
    app.state.viewer_height = 20;
    app.state.content_scroll = 30;
    app.handle_key(KeyCode::Char('e'));
    let request = app.take_edit_request().unwrap();

    // What the user did in the editor.
    fs::write(&path, format!("edited\n{}", numbered(100))).unwrap();
    app.finish_edit(&request, success());

    assert!(app.state.file_content.starts_with("edited\n"));
    assert_eq!(app.state.content_scroll, 30);
    assert!(app.state.status_message.starts_with("Reloaded"));
}

#[test]
fn an_editor_that_never_started_leaves_the_session_alone() {
    let path = scratch("missing-editor.txt", &numbered(10));
    let mut app = app_on(&path);
    app.handle_key(KeyCode::Char('e'));
    let request = app.take_edit_request().unwrap();

    fs::write(&path, "changed behind our back\n").unwrap();
    let not_found = Err(io::Error::from(io::ErrorKind::NotFound));
    app.finish_edit(&request, not_found);

    assert!(app.state.file_content.starts_with("line 1"));
    assert!(app.state.status_message.contains("$EDITOR"));
}
