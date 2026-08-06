//! Mouse routing, vim navigation and the viewer's position indicator.
//!
//! Mouse tests have to draw first: hit regions are measured during rendering,
//! so a click means nothing until a frame has established where things are.

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use flerp::app::App;
use flerp::app_structs::{
    InputMode, TAB_DASHBOARD, TAB_MEDIA, TAB_SEARCH, TAB_SETTINGS, TAB_VIEWER,
};
use flerp::media::{MediaItem, MediaRenderer};
use flerp::ui_components::ui;
use image::{DynamicImage, RgbImage};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 40;

fn app_with_lines(count: usize) -> App {
    let mut app = App::new();
    app.state.file_content = (1..=count)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.state.file_name = "sample.txt".to_string();
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

fn ctrl(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn wheel(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn click(column: u16, row: u16) -> MouseEvent {
    wheel(MouseEventKind::Down(MouseButton::Left), column, row)
}

fn type_keys(app: &mut App, keys: &str) {
    for c in keys.chars() {
        app.handle_key(KeyCode::Char(c));
    }
}

#[test]
fn vim_movement_keys_scroll_the_viewer() {
    let mut app = app_with_lines(500);
    app.state.current_tab = TAB_VIEWER;
    draw(&mut app);

    app.handle_key(KeyCode::Char('j'));
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.state.content_scroll, 2);

    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.state.content_scroll, 1);

    app.handle_key(KeyCode::Char('G'));
    assert!(app.state.content_scroll > 400, "G should reach the end");

    app.handle_key(KeyCode::Char('g'));
    assert_eq!(app.state.content_scroll, 0, "g should return to the top");
}

#[test]
fn control_keys_step_by_half_and_whole_screens() {
    let mut app = app_with_lines(500);
    app.state.current_tab = TAB_VIEWER;
    draw(&mut app);

    let height = app.state.viewer_height;
    assert!(height > 2, "the test terminal should give the viewer room");

    app.handle_key_event(ctrl('d'));
    assert_eq!(app.state.content_scroll, height / 2);

    app.handle_key_event(ctrl('f'));
    assert_eq!(app.state.content_scroll, height / 2 + height);

    app.handle_key_event(ctrl('u'));
    assert_eq!(app.state.content_scroll, height);

    app.handle_key_event(ctrl('b'));
    assert_eq!(app.state.content_scroll, 0);
}

#[test]
fn vim_keys_move_the_selection_in_list_modes() {
    let mut app = app_with_lines(50);
    app.set_search_query("line");
    app.state.current_tab = TAB_SEARCH;

    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.state.selected_result, 1);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.state.selected_result, 0);

    app.state.current_tab = TAB_SETTINGS;
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.state.settings_selection, 1);
}

#[test]
fn n_steps_through_matches_and_wraps_around() {
    let mut app = app_with_lines(4);
    app.set_search_query("line");
    assert_eq!(app.state.search_results.len(), 4);

    app.state.current_tab = TAB_VIEWER;
    app.handle_key(KeyCode::Char('n'));
    assert_eq!(app.state.selected_result, 1);
    // Stepping a match follows it into the viewer rather than only moving a
    // highlight in a list the user may not be looking at.
    assert_eq!(app.state.current_tab, TAB_VIEWER);

    app.handle_key(KeyCode::Char('N'));
    app.handle_key(KeyCode::Char('N'));
    assert_eq!(
        app.state.selected_result, 3,
        "stepping back past the first match should wrap to the last"
    );

    app.handle_key(KeyCode::Char('n'));
    assert_eq!(
        app.state.selected_result, 0,
        "stepping past the last match should wrap to the first"
    );
}

#[test]
fn n_without_matches_says_so_instead_of_moving() {
    let mut app = app_with_lines(10);
    app.state.current_tab = TAB_VIEWER;
    app.state.content_scroll = 4;

    app.handle_key(KeyCode::Char('n'));

    assert_eq!(app.state.content_scroll, 4);
    assert!(
        app.state.status_message.contains("No matches"),
        "status was {:?}",
        app.state.status_message
    );
}

#[test]
fn the_goto_prompt_takes_digits_and_jumps() {
    let mut app = app_with_lines(500);
    draw(&mut app);

    app.handle_key(KeyCode::Char(':'));
    assert_eq!(app.state.input_mode, InputMode::Goto);

    type_keys(&mut app, "12x0");
    assert_eq!(
        app.state.goto_buffer, "120",
        "letters should not reach the buffer"
    );

    app.handle_key(KeyCode::Enter);
    assert_eq!(app.state.input_mode, InputMode::Normal);
    assert_eq!(app.state.current_tab, TAB_VIEWER);
    assert_eq!(app.state.content_scroll, 119);
    assert!(app.state.goto_buffer.is_empty());
}

#[test]
fn the_goto_prompt_clamps_past_the_end_and_says_so() {
    let mut app = app_with_lines(20);
    draw(&mut app);

    app.handle_key(KeyCode::Char(':'));
    type_keys(&mut app, "999");
    app.handle_key(KeyCode::Enter);

    assert!(
        app.state.status_message.contains("past the end"),
        "status was {:?}",
        app.state.status_message
    );
}

#[test]
fn escape_cancels_the_goto_prompt_without_quitting() {
    let mut app = app_with_lines(20);
    app.handle_key(KeyCode::Char(':'));
    type_keys(&mut app, "5");

    assert!(
        app.handle_key(KeyCode::Esc),
        "Esc in a prompt cancels it, it does not quit"
    );
    assert_eq!(app.state.input_mode, InputMode::Normal);
    assert!(app.state.goto_buffer.is_empty());
    assert_eq!(app.state.content_scroll, 0);
}

#[test]
fn command_keys_do_not_leak_into_the_search_query() {
    let mut app = app_with_lines(20);
    app.handle_key(KeyCode::Char('/'));

    type_keys(&mut app, "q:n");

    assert_eq!(app.state.search_query, "q:n");
    assert_eq!(app.state.input_mode, InputMode::Search);
}

#[test]
fn the_wheel_scrolls_the_content_in_whole_notches() {
    let mut app = app_with_lines(500);
    app.state.current_tab = TAB_VIEWER;
    draw(&mut app);

    let inside = app.state.hit.viewer;
    app.handle_mouse(wheel(
        MouseEventKind::ScrollDown,
        inside.x + 2,
        inside.y + 2,
    ));
    assert_eq!(app.state.content_scroll, 3);

    app.handle_mouse(wheel(MouseEventKind::ScrollUp, inside.x + 2, inside.y + 2));
    assert_eq!(app.state.content_scroll, 0);
}

#[test]
fn the_wheel_over_a_list_moves_its_selection() {
    let mut app = app_with_lines(50);
    app.set_search_query("line");
    app.state.current_tab = TAB_SEARCH;
    draw(&mut app);

    let list = app.state.hit.search_results;
    assert!(list.height > 0, "the match list should have been measured");

    app.handle_mouse(wheel(MouseEventKind::ScrollDown, list.x + 1, list.y + 1));
    assert_eq!(app.state.selected_result, 1);
    assert_eq!(
        app.state.content_scroll, 0,
        "a wheel over the list must not also scroll the file"
    );
}

#[test]
fn clicking_a_tab_switches_to_it() {
    let mut app = app_with_lines(20);
    draw(&mut app);

    let (start, end) = app.state.hit.tabs[TAB_MEDIA];
    assert!(end > start, "tab hitboxes should have width");

    app.handle_mouse(click(start + 1, app.state.hit.tab_row));
    assert_eq!(app.state.current_tab, TAB_MEDIA);

    let (start, _) = app.state.hit.tabs[TAB_ANALYZE_INDEX];
    app.handle_mouse(click(start + 1, app.state.hit.tab_row));
    assert_eq!(app.state.current_tab, TAB_ANALYZE_INDEX);
}

const TAB_ANALYZE_INDEX: usize = 3;

#[test]
fn clicking_a_match_selects_the_row_that_was_drawn_there() {
    let mut app = app_with_lines(50);
    app.set_search_query("line");
    app.state.current_tab = TAB_SEARCH;
    draw(&mut app);

    let list = app.state.hit.search_results;
    app.handle_mouse(click(list.x + 3, list.y + 4));

    assert_eq!(app.state.selected_result, 4);
}

#[test]
fn clicking_past_the_last_match_changes_nothing() {
    let mut app = app_with_lines(50);
    app.set_search_query("line 7");
    app.state.current_tab = TAB_SEARCH;
    draw(&mut app);

    assert_eq!(app.state.search_results.len(), 1);
    let list = app.state.hit.search_results;
    app.handle_mouse(click(list.x + 3, list.y + 6));

    assert_eq!(app.state.selected_result, 0);
}

#[test]
fn clicking_an_image_row_picks_that_image() {
    let mut app = app_with_lines(20);
    app.state.media = (0..3)
        .map(|index| MediaItem {
            key: format!("sample#{index}"),
            title: format!("Page {} · image 1", index + 1),
            detail: "8x8 · raw samples · DeviceRGB".to_string(),
            page: Some(index + 1),
            image: DynamicImage::ImageRgb8(RgbImage::new(8, 8)),
        })
        .collect();
    app.state.current_tab = TAB_MEDIA;
    draw(&mut app);

    let list = app.state.hit.media_list;
    // Each entry occupies two rows, so the third one starts on row 4.
    app.handle_mouse(click(list.x + 2, list.y + 4));

    assert_eq!(app.state.selected_media, 2);
}

#[test]
fn leaving_a_mode_retires_its_hit_regions() {
    let mut app = app_with_lines(500);
    app.set_search_query("line");
    app.state.current_tab = TAB_SEARCH;
    draw(&mut app);

    let list = app.state.hit.search_results;
    assert!(list.height > 0);

    // The Analyze mode owns no clickable regions, so the match list must stop
    // catching events aimed at the pane that replaced it.
    app.state.current_tab = TAB_ANALYZE_INDEX;
    draw(&mut app);

    assert_eq!(app.state.hit.search_results.height, 0);

    app.handle_mouse(wheel(MouseEventKind::ScrollDown, list.x + 1, list.y + 1));
    assert_eq!(app.state.selected_result, 0);
    assert_eq!(
        app.state.content_scroll, 3,
        "the wheel should fall through to the content instead"
    );
}

#[test]
fn a_click_while_a_prompt_is_open_is_ignored() {
    let mut app = app_with_lines(20);
    draw(&mut app);
    let (start, _) = app.state.hit.tabs[TAB_MEDIA];

    app.handle_key(KeyCode::Char('/'));
    app.handle_mouse(click(start + 1, app.state.hit.tab_row));

    assert_eq!(
        app.state.current_tab, TAB_SEARCH,
        "the prompt covers the panes, so clicks behind it must not act"
    );
}

#[test]
fn the_viewer_draws_a_scrollbar_with_match_marks() {
    let mut app = app_with_lines(500);
    app.set_search_query("line 4");
    app.state.current_tab = TAB_VIEWER;

    let terminal = draw(&mut app);
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();

    assert!(rendered.contains('█'), "expected a scrollbar thumb");
    assert!(rendered.contains('▪'), "expected match marks on the track");
}

#[test]
fn a_file_that_fits_on_screen_gets_no_scrollbar() {
    let mut app = app_with_lines(3);
    app.state.current_tab = TAB_VIEWER;

    let terminal = draw(&mut app);
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();

    assert!(
        !rendered.contains('█'),
        "nothing to scroll, so nothing to indicate"
    );
}

#[test]
fn the_footer_drops_whole_hints_rather_than_clipping_one() {
    for width in [40, 60, 80, 120, 200] {
        let mut app = app_with_lines(20);
        let mut terminal = Terminal::new(TestBackend::new(width, HEIGHT)).unwrap();
        let mut media = MediaRenderer::halfblocks();
        terminal
            .draw(|frame| ui(frame, &mut app.state, &mut media))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let row = HEIGHT - 3;
        let footer: String = (0..width).map(|x| buffer[(x, row)].symbol()).collect();
        let hints = footer.trim().trim_matches('│').trim();

        assert!(
            hints.starts_with("q quit"),
            "width {width} lost the first hint: {hints:?}"
        );
        // Every rendered hint must be one of the whole ones, never a fragment.
        for hint in hints.split(" | ") {
            assert!(
                !hint.is_empty() && hint.chars().next().is_some_and(|c| !c.is_whitespace()),
                "width {width} produced a partial hint in {hints:?}"
            );
        }
        assert!(
            !hints.ends_with("^d/") && !hints.ends_with('^'),
            "width {width} clipped a hint mid-word: {hints:?}"
        );
    }
}

#[test]
fn the_dashboard_preview_scrolls_too() {
    let mut app = app_with_lines(500);
    app.state.current_tab = TAB_DASHBOARD;
    draw(&mut app);

    app.handle_key(KeyCode::Char('j'));
    assert_eq!(
        app.state.content_scroll, 1,
        "the preview reads the same offset as the viewer, so it should move"
    );

    app.handle_key(KeyCode::Char('G'));
    assert!(app.state.content_scroll > 0);
}
