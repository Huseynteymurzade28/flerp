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

    /// A directory that exists, ready to have files written into it.
    fn created(name: &str) -> Self {
        let directory = Self::new(name);
        fs::create_dir_all(&directory.path).expect("create fixture directory");
        directory
    }

    /// Write a file at a path relative to this directory, making any parent
    /// directories the path implies.
    fn write(&self, relative: &str, contents: &[u8]) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(&path, contents).expect("write fixture file");
        path
    }

    fn as_str(&self) -> &str {
        self.path.to_str().expect("utf-8 fixture path")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn request(file: &str) -> HeadlessRequest {
    HeadlessRequest {
        targets: vec![file.to_string()],
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

/// A directory holding the cases a walk has to get right: two matching files, a
/// match one level down, a hidden file, and a binary that happens to contain the
/// query as readable bytes.
fn corpus(name: &str) -> TempDir {
    let directory = TempDir::created(name);
    directory.write("alpha.txt", b"needle in alpha\nnothing here\n");
    directory.write("beta.md", b"nothing\nneedle in beta, needle twice\n");
    directory.write("nested/deep/gamma.txt", b"needle in gamma\n");
    directory.write("no-match.txt", b"quiet file\n");
    directory.write(".hidden.txt", b"needle in a dotfile\n");
    directory.write("blob.bin", b"needle\0binary payload\n");
    directory
}

fn error_of(request: &HeadlessRequest) -> String {
    let mut buffer = Vec::new();
    run(request, &mut buffer)
        .expect_err("expected the run to fail")
        .to_string()
}

#[test]
fn a_directory_search_walks_into_subdirectories() {
    let directory = corpus("walk");
    let mut request = request(directory.as_str());
    request.search = Some("needle".to_string());

    let output = capture(&request);
    let lines: Vec<&str> = output.lines().collect();

    let expected = |relative: &str, suffix: &str| {
        format!("{}:{suffix}", directory.path.join(relative).display())
    };

    assert_eq!(lines.len(), 3, "output was {output:?}");
    // Paths are sorted, so the same directory reports in the same order twice.
    assert_eq!(lines[0], expected("alpha.txt", "1:needle in alpha"));
    assert_eq!(
        lines[1],
        expected("beta.md", "2:needle in beta, needle twice")
    );
    assert_eq!(
        lines[2],
        expected("nested/deep/gamma.txt", "1:needle in gamma")
    );
}

#[test]
fn a_directory_search_leaves_hidden_files_and_binaries_alone() {
    let directory = corpus("filter");
    let mut request = request(directory.as_str());
    request.search = Some("needle".to_string());

    let output = capture(&request);

    assert!(!output.contains("hidden"), "output was {output:?}");
    assert!(!output.contains("blob.bin"), "output was {output:?}");
}

#[test]
fn naming_a_file_outright_searches_it_whatever_the_walk_would_have_done() {
    let directory = corpus("named");
    let hidden = directory.path.join(".hidden.txt");
    let mut request = request(directory.as_str());
    request.targets = vec![
        hidden.to_str().expect("utf-8 path").to_string(),
        directory.path.join("alpha.txt").display().to_string(),
    ];
    request.search = Some("needle".to_string());

    let output = capture(&request);

    assert_eq!(output.lines().count(), 2, "output was {output:?}");
    assert!(output.contains(".hidden.txt:1:"), "output was {output:?}");
}

#[test]
fn overlapping_targets_do_not_report_the_same_match_twice() {
    let directory = corpus("overlap");
    let mut request = request(directory.as_str());
    request.targets = vec![
        directory.as_str().to_string(),
        directory.path.join("alpha.txt").display().to_string(),
    ];
    request.search = Some("needle".to_string());

    assert_eq!(capture(&request).lines().count(), 3);
}

#[test]
fn json_mode_over_a_directory_records_the_scope_and_the_totals() {
    let directory = corpus("json-dir");
    let mut request = request(directory.as_str());
    request.json = true;
    request.search = Some("needle".to_string());

    let value: Value = serde_json::from_str(&capture(&request)).expect("valid json");

    assert_eq!(value["search"]["query"], "needle");
    assert_eq!(value["roots"][0], directory.as_str());
    // Four searchable files: the dotfile and the binary are not among them.
    assert_eq!(value["files_searched"], 4);
    assert_eq!(value["files_matched"], 3);
    // beta.md matches twice on its one line.
    assert_eq!(value["match_count"], 4);

    let files = value["files"].as_array().expect("files array");
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["kind"], "text");
    assert_eq!(files[1]["match_count"], 2);
    assert_eq!(files[1]["matches"][0]["line_number"], 2);
    // Page is null rather than absent for files with no page structure.
    assert!(files[1]["matches"][0]["page"].is_null());
    // The key is always present, so a consumer needs no special case for a
    // directory where everything was readable.
    assert!(value["skipped"].is_array());
}

#[test]
fn a_directory_without_a_query_says_what_is_missing() {
    let directory = corpus("no-query");
    let request = request(directory.as_str());

    assert!(
        error_of(&request).contains("--search"),
        "the error should point at the missing flag"
    );
}

#[test]
fn the_single_file_output_modes_refuse_a_directory() {
    let directory = corpus("single-only");

    let mut text = request(directory.as_str());
    text.text = true;
    text.search = Some("needle".to_string());
    assert!(error_of(&text).contains("--text"));

    let mut extract = request(directory.as_str());
    extract.extract_images = Some(directory.path.join("out"));
    extract.search = Some("needle".to_string());
    assert!(error_of(&extract).contains("--extract-images"));
}

#[test]
fn a_bad_regex_fails_before_any_file_is_opened() {
    // An empty directory: if the query were only checked while searching a file,
    // there would be no file to check it against and the run would succeed.
    let directory = TempDir::created("empty-dir");
    let mut request = request(directory.as_str());
    request.search = Some("(unclosed".to_string());
    request.search_options.regex_mode = true;

    assert!(!error_of(&request).is_empty());
}

#[test]
fn search_options_apply_across_a_directory() {
    let directory = TempDir::created("dir-options");
    directory.write("upper.txt", b"Needle\n");
    directory.write("lower.txt", b"needle\n");
    let mut request = request(directory.as_str());
    request.search = Some("needle".to_string());

    assert_eq!(capture(&request).lines().count(), 1);

    request.search_options.case_sensitive = false;
    assert_eq!(capture(&request).lines().count(), 2);
}

#[test]
fn an_image_named_in_a_sweep_is_reported_as_skipped_not_searched() {
    // The text flerp shows for an image is a description it wrote itself, which
    // contains the word "Image". Matching on that would be a hit the file does
    // not contain.
    let picture = Fixture::image("sweep-image");
    let directory = TempDir::created("sweep-image-dir");
    directory.write("notes.txt", b"Image of the harbour\n");

    let mut request = request(directory.as_str());
    request.targets = vec![directory.as_str().to_string(), picture.as_str().to_string()];
    request.json = true;
    request.search = Some("Image".to_string());

    let value: Value = serde_json::from_str(&capture(&request)).expect("valid json");

    assert_eq!(value["files_matched"], 1);
    assert_eq!(value["skipped"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        value["skipped"][0]["reason"],
        "an image has no text to search"
    );
}

#[test]
fn a_missing_directory_is_an_error() {
    let mut request = request("/nonexistent/flerp/no-such-directory/");
    request.search = Some("needle".to_string());

    let mut buffer = Vec::new();
    assert!(run(&request, &mut buffer).is_err());
}
