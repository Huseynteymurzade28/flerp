//! Non-interactive output paths.
//!
//! Everything flerp learns about a file is otherwise trapped inside the TUI.
//! These functions write the same information to a stream or to disk, so the
//! analysis can be piped into other tools instead of only being looked at.

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::file_utils::{load_file, LoadedFile};
use crate::media::MediaItem;
use crate::sweep::{self, Plan, Results};
use crate::text_analysis::{
    analyze_content, analyze_structure, extract_repeated_lines, search_with_options, SearchOptions,
};

/// One non-interactive invocation, assembled from the CLI flags.
pub struct HeadlessRequest {
    /// The files and directories named on the command line.
    pub targets: Vec<String>,
    /// Emit a single JSON object describing the file.
    pub json: bool,
    /// Emit the file's extracted plain text verbatim.
    pub text: bool,
    /// Write every embedded image into this directory as PNG.
    pub extract_images: Option<PathBuf>,
    pub search: Option<String>,
    pub search_options: SearchOptions,
    pub keyword_limit: usize,
}

/// What the named targets turned out to be.
enum Target {
    /// One named file, reported in full: the shape flerp has always emitted.
    One(String),
    /// A directory, or several paths. Only searching makes sense across these,
    /// so the plan carries the file list rather than a loaded document.
    Sweep(Plan),
}

/// An image written to disk, reported back so the caller can say what it did.
struct WrittenImage {
    path: PathBuf,
    page: Option<usize>,
    width: u32,
    height: u32,
}

pub fn run(request: &HeadlessRequest, out: &mut impl Write) -> Result<(), Box<dyn Error>> {
    match resolve(&request.targets)? {
        Target::One(file) => run_one(request, &file, out),
        Target::Sweep(plan) => run_sweep(request, &plan, out),
    }
}

/// Work out whether the command line named one file or a set of them.
///
/// One named file is the long-standing shape of the command, and it reports far
/// more than a search: naming a single file keeps that, and everything else is
/// a sweep.
fn resolve(targets: &[String]) -> Result<Target, Box<dyn Error>> {
    if targets.is_empty() {
        return Err("no file or directory was named".into());
    }

    if let [only] = targets {
        if !Path::new(only).is_dir() {
            return Ok(Target::One(only.clone()));
        }
    }

    Ok(Target::Sweep(sweep::plan(targets)?))
}

fn run_one(
    request: &HeadlessRequest,
    file: &str,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let loaded = load_file(file)?;

    // Extraction runs first so its result can be folded into the JSON object
    // rather than printed alongside it, which would leave stdout unparseable.
    let written = match &request.extract_images {
        Some(directory) => write_images(&loaded.media, directory)?,
        None => Vec::new(),
    };

    if request.text {
        write!(out, "{}", loaded.content)?;
        if !loaded.content.ends_with('\n') {
            writeln!(out)?;
        }
        return Ok(());
    }

    if request.json {
        let document = analysis_json(request, file, &loaded, &written)?;
        writeln!(out, "{}", serde_json::to_string_pretty(&document)?)?;
        return Ok(());
    }

    // Plain-text mode: grep-style match lines, then a line per written image.
    if let Some(query) = &request.search {
        let matches = search_with_options(query, &loaded.content, request.search_options)
            .map_err(|error| -> Box<dyn Error> { error.into() })?;
        for entry in &matches {
            writeln!(out, "{}:{}:{}", file, entry.line_number, entry.line)?;
        }
    }

    for image in &written {
        match image.page {
            Some(page) => writeln!(
                out,
                "{} ({}x{}, page {page})",
                image.path.display(),
                image.width,
                image.height
            )?,
            None => writeln!(
                out,
                "{} ({}x{})",
                image.path.display(),
                image.width,
                image.height
            )?,
        }
    }

    Ok(())
}

/// Search every file in the sweep and report the matches.
///
/// Searching is the only thing that means anything across a set of files: there
/// is no single text to write and no one document to describe, so the other
/// output modes say so rather than picking a file arbitrarily.
fn run_sweep(
    request: &HeadlessRequest,
    plan: &Plan,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    if request.text {
        return Err("--text writes one file's text; it has no meaning across several".into());
    }
    if request.extract_images.is_some() {
        return Err("--extract-images works on one file at a time".into());
    }
    let Some(query) = &request.search else {
        return Err(
            "only one file can be opened at a time; pass --search to search across a directory"
                .into(),
        );
    };

    let results = sweep::search(plan, query, request.search_options)
        .map_err(|error| -> Box<dyn Error> { error.into() })?;

    if request.json {
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&sweep_json(request, plan, &results))?
        )?;
        return Ok(());
    }

    for file in &results.files {
        for hit in &file.hits {
            writeln!(
                out,
                "{}:{}:{}",
                file.file.display(),
                hit.line_number,
                hit.line
            )?;
        }
    }

    // Stdout stays grep-parseable, so what could not be read goes to stderr
    // rather than into the middle of the match lines.
    for (path, reason) in &results.skipped {
        eprintln!("flerp: skipped {}: {reason}", path.display());
    }

    Ok(())
}

fn sweep_json(request: &HeadlessRequest, plan: &Plan, results: &Results) -> Value {
    json!({
        "search": {
            "query": results.query,
            "case_sensitive": request.search_options.case_sensitive,
            "regex": request.search_options.regex_mode,
            "whole_word": request.search_options.whole_word,
        },
        "roots": plan.roots,
        "files_searched": results.files_searched,
        "files_matched": results.files.len(),
        "match_count": results.match_count(),
        "files": results
            .files
            .iter()
            .map(|file| {
                json!({
                    "file": file.file,
                    "kind": file.role.label(),
                    "page_count": file.page_count,
                    "match_count": file.match_count(),
                    "matches": file
                        .hits
                        .iter()
                        .map(|hit| {
                            json!({
                                "line_number": hit.line_number,
                                "page": hit.page,
                                "match_count": hit.match_count,
                                "line": hit.line,
                            })
                        })
                        .collect::<Vec<Value>>(),
                })
            })
            .collect::<Vec<Value>>(),
        "skipped": results
            .skipped
            .iter()
            .map(|(path, reason)| json!({ "path": path, "reason": reason }))
            .collect::<Vec<Value>>(),
    })
}

fn write_images(
    media: &[MediaItem],
    directory: &Path,
) -> Result<Vec<WrittenImage>, Box<dyn Error>> {
    fs::create_dir_all(directory)?;

    media
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let path = directory.join(image_file_name(item, index));
            // Always PNG: the images arrive already decoded to pixels, so
            // re-encoding as JPEG would lose data for no gain, and PNG keeps
            // the alpha that /SMask transparency produced.
            item.image.save(&path)?;

            Ok(WrittenImage {
                path,
                page: item.page,
                width: item.image.width(),
                height: item.image.height(),
            })
        })
        .collect()
}

fn image_file_name(item: &MediaItem, index: usize) -> String {
    match item.page {
        Some(page) => format!("page-{page:03}-img-{:02}.png", index + 1),
        None => format!("image-{:02}.png", index + 1),
    }
}

fn analysis_json(
    request: &HeadlessRequest,
    file: &str,
    loaded: &LoadedFile,
    written: &[WrittenImage],
) -> Result<Value, Box<dyn Error>> {
    let structure = analyze_structure(&loaded.content);
    let content = analyze_content(&loaded.content, request.keyword_limit);
    let repeated: Vec<Value> = extract_repeated_lines(&loaded.content, 8)
        .into_iter()
        .map(|(line, count)| json!({ "line": line, "count": count }))
        .collect();

    let mut document = json!({
        "file": file,
        "kind": kind(loaded),
        "language": content.language,
        "stats": structure,
        "readability": content.readability,
        "keywords": content.keywords,
        "phrases": content.phrases,
        "repeated_lines": repeated,
    });

    if let Some(notice) = &loaded.notice {
        document["notice"] = json!(notice);
    }

    if let Some(pdf) = &loaded.document {
        document["pages"] = Value::Array(
            pdf.pages
                .iter()
                .map(|page| {
                    json!({
                        "number": page.number,
                        "start_line": page.start_line,
                        "line_count": page.line_count,
                        "image_count": page.image_count,
                    })
                })
                .collect(),
        );
        document["skipped_images"] = Value::Array(
            pdf.skipped
                .iter()
                .map(|skipped| {
                    json!({
                        "page": skipped.page,
                        "width": skipped.width,
                        "height": skipped.height,
                        "reason": skipped.reason,
                    })
                })
                .collect(),
        );
        if let Some(error) = &pdf.image_error {
            document["image_error"] = json!(error);
        }
    }

    document["images"] = Value::Array(
        loaded
            .media
            .iter()
            .enumerate()
            .map(|(index, item)| {
                json!({
                    "index": index,
                    "title": item.title,
                    "detail": item.detail,
                    "page": item.page,
                    "width": item.image.width(),
                    "height": item.image.height(),
                })
            })
            .collect(),
    );

    if !written.is_empty() {
        document["extracted_images"] = Value::Array(
            written
                .iter()
                .map(|image| json!({ "path": image.path, "page": image.page }))
                .collect(),
        );
    }

    if let Some(query) = &request.search {
        let matches = search_with_options(query, &loaded.content, request.search_options)
            .map_err(|error| -> Box<dyn Error> { error.into() })?;

        document["search"] = json!({
            "query": query,
            "case_sensitive": request.search_options.case_sensitive,
            "regex": request.search_options.regex_mode,
            "whole_word": request.search_options.whole_word,
            "match_count": matches.iter().map(|entry| entry.match_count).sum::<usize>(),
            "matches": matches
                .iter()
                .map(|entry| {
                    json!({
                        "line_number": entry.line_number,
                        "page": page_of(loaded, entry.line_number),
                        "match_count": entry.match_count,
                        "line": entry.line,
                    })
                })
                .collect::<Vec<Value>>(),
        });
    }

    Ok(document)
}

/// The 1-based page a 1-based line falls on, when the file has pages at all.
///
/// A sweep works this out for itself while each document is still loaded; this
/// is the single-file path, which has the whole document in hand throughout.
fn page_of(loaded: &LoadedFile, line_number: usize) -> Option<usize> {
    let pdf = loaded.document.as_ref()?;
    // Line numbers are 1-based; page lookup is not.
    let index = pdf.page_of_line(line_number.saturating_sub(1));
    pdf.pages.get(index).map(|page| page.number)
}

fn kind(loaded: &LoadedFile) -> &'static str {
    if loaded.document.is_some() {
        "pdf"
    } else if !loaded.media.is_empty() {
        "image"
    } else {
        "text"
    }
}
