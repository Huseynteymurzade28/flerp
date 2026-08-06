//! The non-interactive output paths.
//!
//! These run without a terminal on purpose: the point of headless mode is that
//! it works where the TUI cannot.

use std::fs;
use std::path::PathBuf;

use flerp::headless::{run, HeadlessRequest};
use flerp::text_analysis::SearchOptions;
use image::{DynamicImage, RgbImage};
use serde_json::Value;

const SAMPLE: &str = "alpha beta gamma\nbeta gamma\nrepeated line\nrepeated line\n";

/// A temp file that cleans up after itself, so a failing assertion cannot leave
/// litter behind in the temp directory.
struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn text(name: &str, contents: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("flerp-headless-{name}-{}.txt", std::process::id()));
        fs::write(&path, contents).expect("write fixture");
        Self { path }
    }

    fn image(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("flerp-headless-{name}-{}.png", std::process::id()));
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 3, image::Rgb([10, 20, 30])));
        image.save(&path).expect("write image fixture");
        Self { path }
    }

    fn as_str(&self) -> &str {
        self.path.to_str().expect("utf-8 fixture path")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("flerp-headless-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn request(file: &str) -> HeadlessRequest {
    HeadlessRequest {
        file: file.to_string(),
        json: false,
        text: false,
        extract_images: None,
        search: None,
        search_options: SearchOptions {
            case_sensitive: true,
            regex_mode: false,
            whole_word: false,
        },
        keyword_limit: 10,
    }
}

fn capture(request: &HeadlessRequest) -> String {
    let mut buffer = Vec::new();
    run(request, &mut buffer).expect("headless run");
    String::from_utf8(buffer).expect("utf-8 output")
}

#[test]
fn json_mode_emits_one_parseable_object() {
    let fixture = Fixture::text("json", SAMPLE);
    let mut request = request(fixture.as_str());
    request.json = true;

    let value: Value = serde_json::from_str(&capture(&request)).expect("valid json");

    assert_eq!(value["kind"], "text");
    assert_eq!(value["stats"]["lines"], 4);
    assert_eq!(value["stats"]["words"], 9);
    assert_eq!(value["repeated_lines"][0]["line"], "repeated line");
    assert_eq!(value["repeated_lines"][0]["count"], 2);
    // A file with no images still reports the key, so consumers need no special
    // case for it.
    assert_eq!(value["images"].as_array().map(Vec::len), Some(0));
}

#[test]
fn json_mode_reports_search_matches_with_counts() {
    let fixture = Fixture::text("search", SAMPLE);
    let mut request = request(fixture.as_str());
    request.json = true;
    request.search = Some("beta".to_string());

    let value: Value = serde_json::from_str(&capture(&request)).expect("valid json");

    assert_eq!(value["search"]["query"], "beta");
    assert_eq!(value["search"]["match_count"], 2);
    let matches = value["search"]["matches"].as_array().expect("matches");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0]["line_number"], 1);
    // Page is null rather than absent for a file with no page structure.
    assert!(matches[0]["page"].is_null());
}

#[test]
fn text_mode_writes_the_content_verbatim() {
    let fixture = Fixture::text("text", SAMPLE);
    let mut request = request(fixture.as_str());
    request.text = true;

    assert_eq!(capture(&request), SAMPLE);
}

#[test]
fn plain_mode_prints_grep_style_match_lines() {
    let fixture = Fixture::text("grep", SAMPLE);
    let mut request = request(fixture.as_str());
    request.search = Some("gamma".to_string());

    let output = capture(&request);
    let lines: Vec<&str> = output.lines().collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], format!("{}:1:alpha beta gamma", fixture.as_str()));
    assert_eq!(lines[1], format!("{}:2:beta gamma", fixture.as_str()));
}

#[test]
fn search_options_are_honoured() {
    let fixture = Fixture::text("options", "Alpha\nalpha\n");
    let mut request = request(fixture.as_str());
    request.search = Some("alpha".to_string());

    assert_eq!(capture(&request).lines().count(), 1);

    request.search_options.case_sensitive = false;
    assert_eq!(capture(&request).lines().count(), 2);
}

#[test]
fn an_invalid_regex_surfaces_as_an_error_not_a_panic() {
    let fixture = Fixture::text("badregex", SAMPLE);
    let mut request = request(fixture.as_str());
    request.search = Some("(unclosed".to_string());
    request.search_options.regex_mode = true;

    let mut buffer = Vec::new();
    assert!(run(&request, &mut buffer).is_err());
}

#[test]
fn extract_images_writes_png_files_and_reports_them() {
    let fixture = Fixture::image("extract");
    let directory = TempDir::new("extract-out");
    let mut request = request(fixture.as_str());
    request.extract_images = Some(directory.path.clone());

    let output = capture(&request);

    let written = directory.path.join("image-01.png");
    assert!(written.exists(), "expected {} to exist", written.display());
    assert!(output.contains("4x3"), "output was {output:?}");

    // The written file is a real PNG, not an empty placeholder.
    let reopened = image::open(&written).expect("reopen written png");
    assert_eq!((reopened.width(), reopened.height()), (4, 3));
}

#[test]
fn extracting_from_an_image_file_also_lands_in_json() {
    let fixture = Fixture::image("json-extract");
    let directory = TempDir::new("json-extract-out");
    let mut request = request(fixture.as_str());
    request.json = true;
    request.extract_images = Some(directory.path.clone());

    let value: Value = serde_json::from_str(&capture(&request)).expect("valid json");

    assert_eq!(value["kind"], "image");
    assert_eq!(value["images"][0]["width"], 4);
    assert_eq!(
        value["extracted_images"].as_array().map(Vec::len),
        Some(1),
        "extraction should be reported inside the json object, not beside it"
    );
}

#[test]
fn a_missing_file_is_an_error() {
    let request = request("/nonexistent/flerp/definitely-not-here.txt");
    let mut buffer = Vec::new();
    assert!(run(&request, &mut buffer).is_err());
}
