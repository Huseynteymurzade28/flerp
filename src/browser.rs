//! The directory tree pane and the sweep that annotates it.
//!
//! flerp holds one file at a time, so opening a directory needs something that
//! stands beside the loaded file rather than replacing it: this is the list of
//! candidates, and picking from it is what loads the next file.

use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use ratatui::widgets::ListState;

use crate::file_utils::{list_tree, TreeEntry};
use crate::sweep::{self, FileHits, Results};
use crate::text_analysis::SearchOptions;

/// What the tree pane is showing, and what a search found across it.
#[derive(Clone)]
pub struct Browser {
    /// The directory flerp was opened on. The tree never rises above it.
    pub root: PathBuf,
    /// Visible rows, in display order: the flattened tree, re-derived from disk
    /// whenever a folder opens or closes.
    pub entries: Vec<TreeEntry>,
    /// Folders the user has opened. Everything else stays unread, so the cost of
    /// the pane is the cost of what is on screen.
    expanded: HashSet<PathBuf>,
    pub selected: usize,
    pub list_state: ListState,
    /// Matches across the whole tree, once a query has been committed. Kept
    /// whole rather than reduced to counts so the pane can say which file a
    /// match was in and the viewer can still be sent to the line.
    pub results: Option<Results>,
    /// True while the tree has the keyboard rather than the mode pane.
    pub focused: bool,
    /// Why the tree could not be listed, when that is the reason it is empty.
    pub error: Option<String>,
}

impl Browser {
    /// Open the tree on `root`, with its top level listed and nothing expanded.
    pub fn open(root: &Path) -> Result<Self, Box<dyn Error>> {
        let expanded = HashSet::new();
        let entries = list_tree(root, &expanded)?;

        let mut browser = Self {
            root: root.to_path_buf(),
            entries,
            expanded,
            selected: 0,
            list_state: ListState::default(),
            results: None,
            // The tree starts focused because opening flerp on a directory is
            // the user saying they do not yet know which file they want.
            focused: true,
            error: None,
        };
        browser.sync_list_state();
        Ok(browser)
    }

    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.entries.get(self.selected)
    }

    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded.contains(path)
    }

    /// Re-read the tree, keeping the cursor on whatever it was pointing at.
    ///
    /// The path is what the user was looking at; the index is an accident of how
    /// many rows happened to sit above it, and expanding a folder changes that.
    fn refresh(&mut self) {
        let anchor = self.selected_entry().map(|entry| entry.path.clone());

        match list_tree(&self.root, &self.expanded) {
            Ok(entries) => {
                self.entries = entries;
                self.error = None;
            }
            Err(error) => {
                self.entries.clear();
                self.error = Some(error.to_string());
            }
        }

        self.selected = anchor
            .and_then(|path| self.entries.iter().position(|entry| entry.path == path))
            .unwrap_or(0);
        self.sync_list_state();
    }

    pub fn move_selection(&mut self, step: isize) {
        if self.entries.is_empty() {
            return;
        }

        let last = self.entries.len() as isize - 1;
        self.selected = (self.selected as isize + step).clamp(0, last) as usize;
        self.sync_list_state();
    }

    pub fn select_first(&mut self) {
        self.move_selection(-(self.entries.len() as isize));
    }

    pub fn select_last(&mut self) {
        self.move_selection(self.entries.len() as isize);
    }

    /// Put the cursor on a row the user clicked, given the list's own offset.
    pub fn select_row(&mut self, row: usize) {
        let index = self.list_state.offset() + row;
        if index < self.entries.len() {
            self.selected = index;
            self.sync_list_state();
        }
    }

    /// Act on the selected row: open or close a folder, or hand back a file for
    /// the caller to load. Only the caller knows how to load one.
    pub fn activate(&mut self) -> Option<PathBuf> {
        let entry = self.selected_entry()?;
        if !entry.is_dir {
            return Some(entry.path.clone());
        }

        let path = entry.path.clone();
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
        self.refresh();
        None
    }

    /// Open the selected folder, or step into it when it is already open.
    pub fn expand(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if !entry.is_dir {
            return;
        }

        if self.expanded.insert(entry.path.clone()) {
            self.refresh();
        } else {
            self.move_selection(1);
        }
    }

    /// Close the selected folder, or retreat to the parent of whatever this is.
    ///
    /// Collapsing a folder the cursor is not on would be surprising, so a file
    /// row moves the cursor to its folder first and leaves the folder open; a
    /// second press then closes it.
    pub fn collapse(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };

        if entry.is_dir && self.expanded.contains(&entry.path) {
            let path = entry.path.clone();
            self.expanded.remove(&path);
            self.refresh();
            return;
        }

        let Some(parent) = entry.path.parent().map(Path::to_path_buf) else {
            return;
        };
        if let Some(index) = self
            .entries
            .iter()
            .position(|candidate| candidate.path == parent)
        {
            self.selected = index;
            self.sync_list_state();
        }
    }

    /// Search every file under the root and keep the matches for the pane.
    ///
    /// An empty query clears the annotations rather than matching everything:
    /// backing out of a search should leave the tree as it was found.
    pub fn sweep(&mut self, query: &str, options: SearchOptions) -> Result<(), String> {
        if query.is_empty() {
            self.results = None;
            return Ok(());
        }

        let plan = sweep::plan_for(&self.root).map_err(|error| error.to_string())?;
        self.results = Some(sweep::search(&plan, query, options)?);
        Ok(())
    }

    /// How many times the query was found at `path`, when a sweep has run.
    ///
    /// A folder carries the total of everything beneath it, whether or not it is
    /// open. Without that, closing a folder would hide the only evidence that
    /// there is anything in it worth opening.
    pub fn match_count(&self, path: &Path, is_dir: bool) -> Option<usize> {
        let results = self.results.as_ref()?;

        if !is_dir {
            return Some(results.hits_for(path)?.match_count());
        }

        let total: usize = results
            .files
            .iter()
            .filter(|file| file.file.starts_with(path))
            .map(FileHits::match_count)
            .sum();

        (total > 0).then_some(total)
    }

    /// A one-line summary of the last sweep, for the status bar.
    pub fn sweep_summary(&self) -> Option<String> {
        let results = self.results.as_ref()?;
        Some(format!(
            "{} match(es) in {} of {} file(s) under {}",
            results.match_count(),
            results.files.len(),
            results.files_searched,
            self.root.display()
        ))
    }

    fn sync_list_state(&mut self) {
        self.list_state.select(if self.entries.is_empty() {
            None
        } else {
            Some(self.selected)
        });
    }
}
