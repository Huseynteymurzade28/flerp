//! Searching across several files at once.
//!
//! Both the shell and the file browser need this, and they must agree about what
//! gets opened and what a match means, so the rules live here rather than in
//! either caller.

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::file_utils::{collect_searchable, load_for_search, FileRole};
use crate::text_analysis::{search_with_options, SearchOptions};

/// The files a sweep will open, worked out before anything is read.
pub struct Plan {
    /// What the caller asked for, kept so a report can say what was searched.
    pub roots: Vec<String>,
    pub files: Vec<PathBuf>,
    /// Directories the walk was not allowed to look inside, with the reason.
    pub unreadable: Vec<(PathBuf, String)>,
}

/// One line that matched, with the page it sits on for paged documents.
#[derive(Clone)]
pub struct Hit {
    pub line_number: usize,
    pub page: Option<usize>,
    pub match_count: usize,
    pub line: String,
}

/// The matches found in one file.
#[derive(Clone)]
pub struct FileHits {
    pub file: PathBuf,
    pub role: FileRole,
    /// Page count, known because the sweep had the document open anyway.
    pub page_count: Option<usize>,
    pub hits: Vec<Hit>,
}

impl FileHits {
    pub fn match_count(&self) -> usize {
        self.hits.iter().map(|hit| hit.match_count).sum()
    }
}

/// What a sweep found, and what it could not read.
#[derive(Clone)]
pub struct Results {
    /// The query these results came from, so a caller can tell whether they
    /// still describe what the user has since typed.
    pub query: String,
    /// Files with at least one match, in the plan's order.
    pub files: Vec<FileHits>,
    /// How many files were opened, matched or not.
    pub files_searched: usize,
    pub skipped: Vec<(PathBuf, String)>,
}

impl Results {
    pub fn match_count(&self) -> usize {
        self.files.iter().map(FileHits::match_count).sum()
    }

    /// Matches in one file, if it had any. The browser asks this per row.
    pub fn hits_for(&self, path: &Path) -> Option<&FileHits> {
        self.files.iter().find(|file| file.file == path)
    }
}

/// Work out which files a set of named paths covers.
///
/// A path named by hand is included whether or not the walk's own filter would
/// have picked it up: naming a file is the caller saying they want that file.
pub fn plan(targets: &[String]) -> Result<Plan, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut unreadable = Vec::new();

    for target in targets {
        let path = Path::new(target);
        if path.is_dir() {
            let found = collect_searchable(path)?;
            files.extend(found.files);
            unreadable.extend(found.unreadable);
        } else {
            files.push(path.to_path_buf());
        }
    }

    // Overlapping targets — `flerp docs docs/report.pdf` — would otherwise
    // report the same matches twice.
    files.sort();
    files.dedup();

    Ok(Plan {
        roots: targets.to_vec(),
        files,
        unreadable,
    })
}

/// A plan for one directory, which is what the file browser sweeps.
pub fn plan_for(root: &Path) -> Result<Plan, Box<dyn Error>> {
    plan(&[root.to_string_lossy().into_owned()])
}

/// Search every file in the plan.
///
/// The query is compiled before anything is opened, so a bad regex is reported
/// once as the caller's mistake rather than blamed on whichever file happened to
/// be reached first — and reported at all, which searching an empty directory
/// would otherwise skip.
pub fn search(plan: &Plan, query: &str, options: SearchOptions) -> Result<Results, String> {
    search_with_options(query, "", options)?;

    let mut files = Vec::new();
    let mut skipped = plan.unreadable.clone();
    let mut searched = 0usize;

    for path in &plan.files {
        let Some(name) = path.to_str() else {
            skipped.push((path.clone(), "path is not valid UTF-8".to_string()));
            continue;
        };

        let loaded = match load_for_search(name) {
            Ok(loaded) => loaded,
            Err(error) => {
                skipped.push((path.clone(), error.to_string()));
                continue;
            }
        };
        searched += 1;

        let found = search_with_options(query, &loaded.content, options)?;
        if found.is_empty() {
            continue;
        }

        // Page numbers are resolved here, while the document is still loaded, so
        // nothing PDF-sized — least of all its decoded images — has to stay in
        // memory for the rest of the sweep.
        let page_of = |line_number: usize| -> Option<usize> {
            let pdf = loaded.document.as_ref()?;
            // Line numbers are 1-based; page lookup is not.
            let index = pdf.page_of_line(line_number.saturating_sub(1));
            pdf.pages.get(index).map(|page| page.number)
        };

        let hits = found
            .into_iter()
            .map(|entry| Hit {
                page: page_of(entry.line_number),
                line_number: entry.line_number,
                match_count: entry.match_count,
                line: entry.line,
            })
            .collect();

        files.push(FileHits {
            role: if loaded.document.is_some() {
                FileRole::Pdf
            } else {
                FileRole::Text
            },
            page_count: loaded.document.as_ref().map(|pdf| pdf.page_count()),
            file: path.clone(),
            hits,
        });
    }

    Ok(Results {
        query: query.to_string(),
        files,
        files_searched: searched,
        skipped,
    })
}
