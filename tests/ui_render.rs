//! Smoke tests for the workspace chrome: every mode must draw, and the Media
//! mode must put real image cells on screen rather than an empty pane.

use flerp::app_structs::{
    AppState, TAB_ANALYZE, TAB_COUNT, TAB_DASHBOARD, TAB_MEDIA, TAB_SEARCH, TAB_SETTINGS,
    TAB_VIEWER,
};
use flerp::media::{MediaItem, MediaRenderer};
use flerp::ui_components::ui;
use image::{DynamicImage, Rgb, RgbImage};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn swatch() -> DynamicImage {
    let mut image = RgbImage::new(8, 8);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = if (x + y) % 2 == 0 {
            Rgb([220, 40, 40])
        } else {
            Rgb([40, 60, 220])
        };
    }
    DynamicImage::ImageRgb8(image)
}

fn state_with_media() -> AppState {
    let mut state = AppState {
        file_content: (1..=200)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n"),
        file_name: "sample.pdf".to_string(),
        ..AppState::default()
    };
    state.media = vec![MediaItem {
        key: "sample#0".to_string(),
        title: "Page 1 · image 1".to_string(),
        detail: "8x8 · raw samples · DeviceRGB".to_string(),
        page: Some(1),
        image: swatch(),
    }];
    state
}

#[test]
fn every_mode_draws_without_panicking() {
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    let mut media = MediaRenderer::halfblocks();
    let mut state = state_with_media();

    for tab in [
        TAB_DASHBOARD,
        TAB_SEARCH,
        TAB_VIEWER,
        TAB_ANALYZE,
        TAB_MEDIA,
        TAB_SETTINGS,
    ] {
        state.current_tab = tab;
        terminal
            .draw(|frame| ui(frame, &mut state, &mut media))
            .unwrap_or_else(|error| panic!("tab {tab} failed to draw: {error}"));
    }

    // The tab bar must list exactly the modes the Tab key cycles through.
    state.current_tab = TAB_DASHBOARD;
    terminal
        .draw(|frame| ui(frame, &mut state, &mut media))
        .unwrap();
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    for label in ["Dashboard", "Search", "Viewer", "Analyze", "Media", "Settings"] {
        assert!(rendered.contains(label), "tab bar is missing {label}");
    }
    assert_eq!(TAB_COUNT, 6);
}

#[test]
fn viewer_reports_the_window_it_actually_drew() {
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    let mut media = MediaRenderer::halfblocks();
    let mut state = state_with_media();
    state.current_tab = TAB_VIEWER;

    terminal
        .draw(|frame| ui(frame, &mut state, &mut media))
        .unwrap();

    // The viewer sizes itself to the pane, so paging keys can step by a screenful.
    assert!(
        state.viewer_height > 1,
        "viewer height was never measured: {}",
        state.viewer_height
    );

    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(rendered.contains(&format!("1-{} of 200", state.viewer_height)));
    // The last line inside the window is on screen, so the report is honest.
    assert!(rendered.contains(&format!("line {}", state.viewer_height)));
}

#[test]
fn media_mode_paints_the_image_area() {
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    let mut media = MediaRenderer::halfblocks();
    let mut state = state_with_media();
    state.current_tab = TAB_MEDIA;

    terminal
        .draw(|frame| ui(frame, &mut state, &mut media))
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let rendered: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
    assert!(rendered.contains("Page 1 · image 1"));
    assert!(rendered.contains("Unicode half-blocks"));

    // Half-blocks paint the swatch's colours into the preview pane; a blank pane
    // would mean the image never reached the screen.
    let painted = buffer
        .content()
        .iter()
        .any(|cell| cell.symbol() == "▀" || cell.symbol() == "▄");
    assert!(painted, "no image cells were drawn in the preview pane");
}

#[test]
fn media_mode_explains_itself_when_there_is_nothing_to_show() {
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    let mut media = MediaRenderer::halfblocks();
    let mut state = state_with_media();
    state.media.clear();
    state.current_tab = TAB_MEDIA;

    terminal
        .draw(|frame| ui(frame, &mut state, &mut media))
        .unwrap();

    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(rendered.contains("No images in this file"));
}
