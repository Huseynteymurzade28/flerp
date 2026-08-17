//! Renders the TUI to a stream of frames for the README's images.
//!
//! Screenshots taken by hand go stale the moment a pane moves, and nobody can
//! tell from the picture which version it was. Driving the app the way the tests
//! do and dumping what it drew makes the images a build artefact: the same
//! keystrokes produce the same frames every time, on any machine, with no
//! terminal or window manager in the way.
//!
//! Frames go out as JSON — one cell per character, with the colours the palette
//! actually chose — for `assets/render.py` to turn into PNGs and a GIF.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;
use flerp::app::App;
use flerp::media::MediaRenderer;
use flerp::ui_components::ui;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use serde_json::{json, Value};

const WIDTH: u16 = 96;
const HEIGHT: u16 = 30;

/// A directory that looks like something worth searching, so the images show
/// the tool doing its job rather than a folder of `foo.txt`.
fn corpus(root: &Path) -> std::io::Result<()> {
    let _ = fs::remove_dir_all(root);

    let files: &[(&str, &str)] = &[
        (
            "contracts/2026/harbour-extension.txt",
            "HARBOUR EXTENSION WORKS\n\nThe contractor shall complete the harbour extension works\nby the thirtieth of June. An invoice for the first stage\nfalls due on completion of the survey.\n\nRetention of five per cent applies to each invoice until\nthe harbour extension works are certified.\n\nNo invoice is payable while the works remain uncertified.\n",
        ),
        (
            "contracts/2026/quarry-haulage.txt",
            "QUARRY HAULAGE\n\nHaulage from the quarry to the harbour is charged per tonne.\nAn invoice is raised monthly.\n",
        ),
        (
            "contracts/2025/dredging.txt",
            "DREDGING OF THE APPROACH CHANNEL\n\nThe approach channel shall be dredged to the depth shown on\nthe survey drawings. No invoice is payable before the\nsoundings have been agreed.\n",
        ),
        (
            "notes/site-visit.md",
            "# Site visit, 3 March\n\nWalked the harbour wall with the resident engineer. The\nextension works are two weeks behind. No invoice yet for the\ncrane hire.\n\nSoundings were being taken off the north quay.\n",
        ),
        (
            "notes/telephone.md",
            "# Telephone note\n\nCalled about the outstanding invoice. Promised to send the\ncertificate by Friday.\n",
        ),
        ("README.md", "Project files for the harbour job.\n"),
        ("logo.png", "\u{89}PNG\r\n"),
    ];

    for (relative, contents) in files {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("a parent"))?;
        fs::write(path, contents)?;
    }

    Ok(())
}

/// One frame, flattened to the three things a renderer needs per cell.
fn capture(app: &mut App) -> Value {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("a terminal");
    let mut media = MediaRenderer::halfblocks();
    terminal
        .draw(|frame| ui(frame, &mut app.state, &mut media))
        .expect("a frame");

    let buffer = terminal.backend().buffer();
    let cells: Vec<Value> = buffer
        .content()
        .iter()
        .map(|cell| {
            json!({
                "s": cell.symbol(),
                "fg": rgb(cell.fg),
                "bg": rgb(cell.bg),
                "b": cell.modifier.contains(Modifier::BOLD),
            })
        })
        .collect();

    json!({ "width": WIDTH, "height": HEIGHT, "cells": cells })
}

/// Colours as the renderer will want them. Every palette entry is already RGB;
/// `Reset` is the one that is not, and it means "whatever the theme's ground is".
fn rgb(color: Color) -> Value {
    match color {
        Color::Rgb(r, g, b) => json!([r, g, b]),
        _ => Value::Null,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| {
        std::env::temp_dir()
            .join("flerp-shots")
            .display()
            .to_string()
    }));
    corpus(&root.join("harbour"))?;
    // The header shows the path of whatever is loaded, so the demo runs from
    // beside the corpus and opens it by its bare name. An absolute temp path
    // would put a machine-specific string across the top of every image.
    std::env::set_current_dir(&root)?;

    let mut app = App::new();
    app.open_directory(Path::new("harbour"))?;

    let mut frames: Vec<Value> = Vec::new();
    // Held frames make the GIF readable: a keystroke that lands and is gone in
    // one frame reads as a flicker rather than as an action.
    fn shoot(app: &mut App, frames: &mut Vec<Value>, hold: usize) {
        let frame = capture(app);
        for _ in 0..hold {
            frames.push(frame.clone());
        }
    }

    shoot(&mut app, &mut frames, 10);

    // Open `contracts`, then the year inside it, so the images show the tree
    // being walked rather than a flat listing.
    app.handle_key(KeyCode::Enter);
    shoot(&mut app, &mut frames, 8);
    for _ in 0..2 {
        app.handle_key(KeyCode::Char('j'));
        shoot(&mut app, &mut frames, 4);
    }
    app.handle_key(KeyCode::Enter);
    shoot(&mut app, &mut frames, 10);

    // Search the whole tree. Every keystroke is a frame, so the query types out.
    app.handle_key(KeyCode::Char('/'));
    shoot(&mut app, &mut frames, 5);
    for character in "invoice".chars() {
        app.handle_key(KeyCode::Char(character));
        shoot(&mut app, &mut frames, 4);
    }
    app.handle_key(KeyCode::Enter);
    shoot(&mut app, &mut frames, 16);

    // Pick the file with the most matches and land on the first of them.
    app.handle_key(KeyCode::Char('j'));
    shoot(&mut app, &mut frames, 6);
    app.handle_key(KeyCode::Enter);
    shoot(&mut app, &mut frames, 22);

    // The still is where the animation ends: the tree carrying its counts, the
    // file open, and the viewer already sitting on the match.
    let still = frames.last().cloned().expect("a frame");

    let out = root.join("frames.json");
    let mut file = fs::File::create(&out)?;
    write!(file, "{}", json!({ "still": still, "frames": frames }))?;

    println!("{} frames -> {}", frames.len(), out.display());
    Ok(())
}
