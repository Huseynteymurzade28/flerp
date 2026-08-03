//! Key handling for the paged viewer and the Media mode.
//!
//! These tests deliberately avoid the keys that persist settings, so running the
//! suite never touches the user's config file.

use std::sync::Arc;

use crossterm::event::KeyCode;
use flerp::app::App;
use flerp::app_structs::{TAB_DASHBOARD, TAB_MEDIA, TAB_SETTINGS, TAB_VIEWER};
use flerp::media::MediaItem;
use flerp::pdf_doc::{PdfDocument, PdfPage};
use image::{DynamicImage, RgbImage};

fn page(number: usize, start_line: usize, line_count: usize, image_count: usize) -> PdfPage {
    PdfPage {
        number,
        start_line,
        line_count,
        image_count,
    }
}

/// Three pages of 20 lines each, with one image on page 3.
fn paged_app() -> App {
    let mut app = App::new();
    app.state.file_content = (1..=60)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.state.file_name = "paged.pdf".to_string();
    app.state.viewer_height = 10;
    app.state.document = Some(Arc::new(PdfDocument {
        text: app.state.file_content.clone(),
        pages: vec![page(1, 0, 20, 0), page(2, 20, 20, 0), page(3, 40, 20, 1)],
        images: Vec::new(),
        skipped: Vec::new(),
        image_error: None,
    }));
    app.state.media = vec![MediaItem {
        key: "paged#0".to_string(),
        title: "Page 3 · image 1".to_string(),
        detail: "4x4".to_string(),
        page: Some(3),
        image: DynamicImage::ImageRgb8(RgbImage::new(4, 4)),
    }];
    app
}

#[test]
fn tab_cycles_through_all_six_modes() {
    let mut app = App::new();
    app.state.current_tab = TAB_DASHBOARD;

    let mut seen = vec![app.state.current_tab];
    for _ in 0..6 {
        app.handle_key(KeyCode::Tab);
        seen.push(app.state.current_tab);
    }

    assert_eq!(seen, vec![0, 1, 2, 3, 4, 5, 0], "Tab must visit Media too");
    assert_eq!(seen[TAB_MEDIA], TAB_MEDIA);
    assert_eq!(seen[TAB_SETTINGS], TAB_SETTINGS);
}

#[test]
fn bracket_keys_step_whole_pages() {
    let mut app = paged_app();

    app.handle_key(KeyCode::Char(']'));
    assert_eq!(app.state.content_scroll, 20);
    assert_eq!(app.state.current_page(), Some(2));
    assert_eq!(app.state.current_tab, TAB_VIEWER);

    app.handle_key(KeyCode::Char(']'));
    assert_eq!(app.state.content_scroll, 40);
    assert_eq!(app.state.current_page(), Some(3));

    // Already on the last page: stay put rather than scrolling into nothing.
    app.handle_key(KeyCode::Char(']'));
    assert_eq!(app.state.content_scroll, 40);

    app.handle_key(KeyCode::Char('['));
    assert_eq!(app.state.content_scroll, 20);

    app.handle_key(KeyCode::Char('['));
    assert_eq!(app.state.content_scroll, 0);
    app.handle_key(KeyCode::Char('['));
    assert_eq!(app.state.content_scroll, 0, "must not run off the front");
}

#[test]
fn stepping_back_from_mid_page_returns_to_that_page_top() {
    let mut app = paged_app();
    app.state.content_scroll = 27; // partway into page 2

    app.handle_key(KeyCode::Char('['));
    assert_eq!(app.state.content_scroll, 20, "first press snaps to page 2 top");

    app.handle_key(KeyCode::Char('['));
    assert_eq!(app.state.content_scroll, 0, "second press moves to page 1");
}

#[test]
fn page_keys_are_inert_without_a_paged_document() {
    let mut app = App::new();
    app.state.file_content = "one\ntwo\nthree".to_string();

    app.handle_key(KeyCode::Char(']'));

    assert_eq!(app.state.content_scroll, 0);
    assert!(app.state.status_message.contains("paged document"));
}

#[test]
fn media_selection_stays_in_bounds_and_jumps_to_the_right_page() {
    let mut app = paged_app();
    app.state.media.push(MediaItem {
        key: "paged#1".to_string(),
        title: "Page 1 · image 2".to_string(),
        detail: "4x4".to_string(),
        page: Some(1),
        image: DynamicImage::ImageRgb8(RgbImage::new(4, 4)),
    });
    app.state.current_tab = TAB_MEDIA;

    app.handle_key(KeyCode::Up);
    assert_eq!(app.state.selected_media, 0, "cannot select above the first");

    app.handle_key(KeyCode::Down);
    assert_eq!(app.state.selected_media, 1);
    app.handle_key(KeyCode::Down);
    assert_eq!(app.state.selected_media, 1, "cannot select past the last");

    // Enter follows the selected image to the page it sits on.
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.state.current_tab, TAB_VIEWER);
    assert_eq!(app.state.content_scroll, 0);
    assert_eq!(app.state.current_page(), Some(1));

    app.state.current_tab = TAB_MEDIA;
    app.state.selected_media = 0;
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.state.content_scroll, 40);
    assert_eq!(app.state.current_page(), Some(3));
}

#[test]
fn paging_keys_step_by_the_measured_viewer_height() {
    let mut app = paged_app();
    app.state.current_tab = TAB_VIEWER;
    app.state.viewer_height = 12;

    app.handle_key(KeyCode::PageDown);
    assert_eq!(app.state.content_scroll, 12);

    app.handle_key(KeyCode::PageUp);
    assert_eq!(app.state.content_scroll, 0);

    // End stops where the last line is still on screen, not past it.
    app.handle_key(KeyCode::End);
    assert_eq!(app.state.content_scroll, 60 - 12);
}
