use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use std::error::Error;
use std::time::Instant;

use crate::app_structs::{
    AppState, InputMode, TAB_COUNT, TAB_MEDIA, TAB_SEARCH, TAB_SETTINGS, TAB_VIEWER,
};
use crate::file_utils::load_file;
use crate::pdf_doc::PdfDocument;
use crate::settings::AppSettings;
use crate::text_analysis::{
    analyze_content, analyze_structure, extract_repeated_lines, search_with_options, SearchOptions,
};

/// Lines one wheel notch moves the content. Three is the common terminal step;
/// one line per notch makes a wheel feel broken.
const WHEEL_LINES: isize = 3;

/// Index of the last row in the Settings list.
const SETTINGS_LAST: usize = 7;

pub struct App {
    pub state: AppState,
    pub last_tick: Instant,
}

impl App {
    pub fn new() -> Self {
        let mut state = AppState::default();
        AppSettings::load().apply_to_state(&mut state);

        Self {
            state,
            last_tick: Instant::now(),
        }
    }

    pub fn load_file(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let loaded = load_file(file_path)?;

        self.state.file_content = loaded.content;
        self.state.file_name = file_path.to_string();
        self.state.document = loaded.document;
        self.state.media = loaded.media;
        self.state.selected_media = 0;
        self.refresh_analysis();
        self.state.content_scroll = 0;
        self.update_search();

        let summary = match &self.state.document {
            Some(document) => format!(
                "Loaded {} · {} pages · {} images",
                self.state.file_name,
                document.page_count(),
                self.state.media.len()
            ),
            None => format!("Loaded {}", self.state.file_name),
        };
        self.state.status_message = match loaded.notice {
            Some(notice) => format!("{summary} · {notice}"),
            None => summary,
        };

        Ok(())
    }

    fn refresh_analysis(&mut self) {
        self.state.structural_analysis = analyze_structure(&self.state.file_content);

        let content = analyze_content(&self.state.file_content, self.state.keyword_limit);
        self.state.keywords = content.keywords;
        self.state.phrases = content.phrases;
        self.state.readability = content.readability;

        self.state.repeated_lines = extract_repeated_lines(&self.state.file_content, 8);
        self.state.content_scroll = self.state.content_scroll.min(self.max_content_scroll());
    }

    fn update_settings_selection(&mut self, direction: isize) {
        let next =
            (self.state.settings_selection as isize + direction).clamp(0, SETTINGS_LAST as isize);
        self.state.settings_selection = next as usize;
    }

    fn adjust_setting(&mut self, increase: bool) {
        match self.state.settings_selection {
            0 => {
                self.state.theme = if increase {
                    self.state.theme.next()
                } else {
                    self.state.theme.previous()
                };
            }
            1 => {
                let step = if increase { 1 } else { -1 };
                let next = (self.state.keyword_limit as isize + step).clamp(5, 30) as usize;
                if next != self.state.keyword_limit {
                    self.state.keyword_limit = next;
                    self.refresh_analysis();
                }
            }
            2 => {
                let step = if increase { 5 } else { -5 };
                self.state.preview_line_count =
                    (self.state.preview_line_count as isize + step).clamp(10, 200) as usize;
                self.state.content_scroll = self.state.content_scroll.min(self.max_content_scroll());
            }
            3 => self.state.case_sensitive = increase,
            4 => self.state.regex_mode = increase,
            5 => self.state.whole_word = increase,
            6 => self.state.line_numbers = increase,
            7 => self.state.wrap_lines = increase,
            _ => {}
        }

        if (3..=5).contains(&self.state.settings_selection) {
            self.update_search();
        }

        self.persist_settings();
    }

    fn persist_settings(&mut self) {
        match AppSettings::from_state(&self.state).save() {
            Ok(()) => {
                self.state.status_message = "Settings saved to XDG config directory.".to_string();
            }
            Err(error) => {
                self.state.status_message = format!("Could not save settings: {error}");
            }
        }
    }

    fn max_content_scroll(&self) -> usize {
        self.state
            .file_content
            .lines()
            .count()
            .saturating_sub(self.state.viewer_height)
    }

    /// Move the viewer by whole PDF pages. `step` is in pages, not lines.
    fn jump_page(&mut self, step: isize) {
        let Some(document) = self.state.document.clone() else {
            self.state.status_message = "Page jumps need a paged document such as a PDF.".to_string();
            return;
        };
        if document.pages.is_empty() {
            return;
        }

        let current = document.page_of_line(self.state.content_scroll);
        // Stepping back from mid-page returns to the top of the current page
        // first, which is what a reader expects from a "previous page" key.
        let at_page_start = document
            .pages
            .get(current)
            .is_some_and(|page| page.start_line == self.state.content_scroll);
        let target = if step < 0 && !at_page_start {
            current
        } else {
            (current as isize + step).clamp(0, document.pages.len() as isize - 1) as usize
        };

        self.show_page(&document, target);
    }

    /// Move the viewer to a 1-based page number, as `--page` does.
    pub fn goto_page(&mut self, number: usize) {
        let Some(document) = self.state.document.clone() else {
            self.state.status_message =
                "Page jumps need a paged document such as a PDF.".to_string();
            return;
        };

        let Some(index) = document
            .pages
            .iter()
            .position(|page| page.number == number)
        else {
            self.state.status_message = format!(
                "Page {number} is outside this document's {} page(s).",
                document.page_count()
            );
            return;
        };

        self.show_page(&document, index);
    }

    /// Park the viewer at the top of `pages[index]` and report where it landed.
    fn show_page(&mut self, document: &PdfDocument, index: usize) {
        let Some(page) = document.pages.get(index) else {
            return;
        };

        // Land on the page start verbatim; the viewer clamps to the real pane
        // height when it draws, which is the only place that height is known.
        self.state.content_scroll = page.start_line;
        self.state.current_tab = TAB_VIEWER;
        self.state.status_message = format!(
            "Page {} of {} · {} lines · {} image(s)",
            page.number,
            document.pages.len(),
            page.line_count,
            page.image_count
        );
    }

    /// Seed the search box from outside the event loop, as `--search` does.
    pub fn set_search_query(&mut self, query: &str) {
        self.state.search_query = query.to_string();
        self.update_search();
        self.state.current_tab = TAB_SEARCH;
    }

    fn select_media(&mut self, step: isize) {
        if self.state.media.is_empty() {
            return;
        }
        let last = self.state.media.len() as isize - 1;
        let next = (self.state.selected_media as isize + step).clamp(0, last) as usize;
        self.state.selected_media = next;
    }

    /// Jump the viewer to the page holding the selected image.
    fn jump_to_media_page(&mut self) {
        let Some(document) = self.state.document.clone() else {
            return;
        };
        let Some(item) = self.state.media.get(self.state.selected_media) else {
            return;
        };
        let (title, Some(page_number)) = (item.title.clone(), item.page) else {
            return;
        };
        let Some(page) = document
            .pages
            .iter()
            .find(|page| page.number == page_number)
        else {
            return;
        };

        self.state.content_scroll = page.start_line;
        self.state.current_tab = TAB_VIEWER;
        self.state.status_message = format!("Jumped to page {page_number} for {title}");
    }

    fn scroll_content(&mut self, delta: isize) {
        let max_scroll = self.max_content_scroll() as isize;
        let next = (self.state.content_scroll as isize + delta).clamp(0, max_scroll);
        self.state.content_scroll = next as usize;
    }

    fn jump_to_selected_result(&mut self) {
        if let Some(selected) = self.state.search_results.get(self.state.selected_result) {
            let target = selected.line_number.saturating_sub(3);
            self.state.content_scroll = target.min(self.max_content_scroll());
            self.state.current_tab = TAB_VIEWER;
            self.state.status_message = match self.state.current_page() {
                Some(page) => format!(
                    "Jumped to line {} (page {page}) from search results.",
                    selected.line_number
                ),
                None => format!(
                    "Jumped to line {} from search results.",
                    selected.line_number
                ),
            };
        }
    }

    pub fn update_search(&mut self) {
        self.state.search_error = None;

        match search_with_options(
            &self.state.search_query,
            &self.state.file_content,
            SearchOptions {
                case_sensitive: self.state.case_sensitive,
                regex_mode: self.state.regex_mode,
                whole_word: self.state.whole_word,
            },
        ) {
            Ok(results) => {
                self.state.search_results = results;
            }
            Err(error) => {
                self.state.search_results.clear();
                self.state.search_error = Some(error);
            }
        }

        self.state.selected_result = 0;
        self.state.result_list_state.select(if self.state.search_results.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    /// Move the search selection without following it into the viewer.
    fn move_result(&mut self, step: isize) {
        if self.state.search_results.is_empty() {
            return;
        }

        let last = self.state.search_results.len() as isize - 1;
        let next = (self.state.selected_result as isize + step).clamp(0, last) as usize;
        self.state.selected_result = next;
        self.state.result_list_state.select(Some(next));
    }

    /// Step to the next or previous match and follow it in the viewer.
    ///
    /// Wraps at both ends, the way `n` does in a pager: reaching the last match
    /// and being told there are no more is less useful than starting over.
    fn step_match(&mut self, step: isize) {
        if self.state.search_results.is_empty() {
            self.state.status_message =
                "No matches to step through. Press / to search.".to_string();
            return;
        }

        let count = self.state.search_results.len() as isize;
        let next = (self.state.selected_result as isize + step).rem_euclid(count) as usize;
        self.state.selected_result = next;
        self.state.result_list_state.select(Some(next));
        self.jump_to_selected_result();
    }

    /// Put the viewer on a 1-based line number, as the `:` prompt does.
    fn goto_line(&mut self, line: usize) {
        let total = self.state.file_content.lines().count();
        if total == 0 {
            self.state.status_message = "No file loaded.".to_string();
            return;
        }

        let clamped = line.clamp(1, total);
        self.state.content_scroll = clamped.saturating_sub(1).min(self.max_content_scroll());
        self.state.current_tab = TAB_VIEWER;
        self.state.status_message = if clamped == line {
            match self.state.current_page() {
                Some(page) => format!("Jumped to line {clamped} (page {page})."),
                None => format!("Jumped to line {clamped}."),
            }
        } else {
            format!("Line {line} is past the end; stopped at {clamped} of {total}.")
        };
    }

    /// Entry point for an unmodified key. Returns false when the app should quit.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        self.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> bool {
        match self.state.input_mode {
            InputMode::Search => {
                self.handle_search_key(event.code);
                true
            }
            InputMode::Goto => {
                self.handle_goto_key(event.code);
                true
            }
            InputMode::Normal => self.handle_command_key(event),
        }
    }

    fn handle_search_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.state.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                self.state.input_mode = InputMode::Normal;
                self.update_search();
            }
            KeyCode::Backspace => {
                self.state.search_query.pop();
                self.update_search();
            }
            KeyCode::Char(c) => {
                self.state.search_query.push(c);
                self.update_search();
            }
            _ => {}
        }
    }

    fn handle_goto_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.state.input_mode = InputMode::Normal;
                self.state.goto_buffer.clear();
            }
            KeyCode::Enter => {
                let target = self.state.goto_buffer.parse::<usize>().ok();
                self.state.input_mode = InputMode::Normal;
                self.state.goto_buffer.clear();
                match target {
                    Some(line) => self.goto_line(line),
                    None => {
                        self.state.status_message = "The : prompt wants a line number.".to_string()
                    }
                }
            }
            KeyCode::Backspace => {
                self.state.goto_buffer.pop();
            }
            // Digits only. Silently swallowing letters would leave the user
            // typing into a prompt that quietly refuses to accept them.
            KeyCode::Char(c) if c.is_ascii_digit() => self.state.goto_buffer.push(c),
            _ => {}
        }
    }

    fn handle_command_key(&mut self, event: KeyEvent) -> bool {
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_control_key(event.code);
        }

        match vim_alias(event.code) {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('/') => {
                self.state.input_mode = InputMode::Search;
                self.state.current_tab = TAB_SEARCH;
            }
            KeyCode::Char(':') => {
                self.state.input_mode = InputMode::Goto;
                self.state.goto_buffer.clear();
            }
            KeyCode::Tab => {
                self.state.current_tab = (self.state.current_tab + 1) % TAB_COUNT;
            }
            KeyCode::BackTab => {
                self.state.current_tab = (self.state.current_tab + TAB_COUNT - 1) % TAB_COUNT;
            }
            KeyCode::Char('[') => self.jump_page(-1),
            KeyCode::Char(']') => self.jump_page(1),
            KeyCode::Char('n') => self.step_match(1),
            KeyCode::Char('N') => self.step_match(-1),
            KeyCode::Char('c') => {
                self.state.case_sensitive = !self.state.case_sensitive;
                self.update_search();
                self.persist_settings();
            }
            KeyCode::Char('r') => {
                self.state.regex_mode = !self.state.regex_mode;
                self.update_search();
                self.persist_settings();
            }
            KeyCode::Char('w') => {
                self.state.whole_word = !self.state.whole_word;
                self.update_search();
                self.persist_settings();
            }
            KeyCode::Char('l') => {
                self.state.line_numbers = !self.state.line_numbers;
                self.persist_settings();
            }
            KeyCode::Char('z') => {
                self.state.wrap_lines = !self.state.wrap_lines;
                self.persist_settings();
            }
            KeyCode::Enter if self.state.current_tab == TAB_SEARCH => {
                self.jump_to_selected_result();
            }
            KeyCode::Enter if self.state.current_tab == TAB_MEDIA => {
                self.jump_to_media_page();
            }
            KeyCode::Up => match self.state.current_tab {
                TAB_SEARCH => self.move_result(-1),
                TAB_MEDIA => self.select_media(-1),
                TAB_SETTINGS => self.update_settings_selection(-1),
                _ => self.scroll_content(-1),
            },
            KeyCode::Down => match self.state.current_tab {
                TAB_SEARCH => self.move_result(1),
                TAB_MEDIA => self.select_media(1),
                TAB_SETTINGS => self.update_settings_selection(1),
                _ => self.scroll_content(1),
            },
            KeyCode::PageUp if self.state.shows_content() => {
                self.scroll_content(-(self.state.viewer_height as isize));
            }
            KeyCode::PageDown if self.state.shows_content() => {
                self.scroll_content(self.state.viewer_height as isize);
            }
            KeyCode::Home if self.state.shows_content() => self.state.content_scroll = 0,
            KeyCode::End if self.state.shows_content() => {
                self.state.content_scroll = self.max_content_scroll();
            }
            KeyCode::Left if self.state.current_tab == TAB_SETTINGS => self.adjust_setting(false),
            KeyCode::Right if self.state.current_tab == TAB_SETTINGS => self.adjust_setting(true),
            _ => {}
        }

        true
    }

    fn handle_control_key(&mut self, key: KeyCode) -> bool {
        // Half- and full-screen steps are measured against the pane the last
        // draw actually produced, so they match what is on screen.
        let half = (self.state.viewer_height / 2).max(1) as isize;
        let full = self.state.viewer_height as isize;

        match key {
            KeyCode::Char('c') => return false,
            KeyCode::Char('d') => self.scroll_content(half),
            KeyCode::Char('u') => self.scroll_content(-half),
            KeyCode::Char('f') => self.scroll_content(full),
            KeyCode::Char('b') => self.scroll_content(-full),
            _ => {}
        }

        true
    }

    /// Route one mouse event. Returns false when the app should quit, which it
    /// never does; the signature matches [`App::handle_key_event`] so the event
    /// loop can treat both the same way.
    pub fn handle_mouse(&mut self, event: MouseEvent) -> bool {
        // A prompt covers the panes underneath it, so a click there would act on
        // something the user cannot see.
        if self.state.is_typing() {
            return true;
        }

        let position = Position::new(event.column, event.row);
        match event.kind {
            MouseEventKind::ScrollDown => self.scroll_at(position, 1),
            MouseEventKind::ScrollUp => self.scroll_at(position, -1),
            MouseEventKind::Down(MouseButton::Left) => self.click_at(position),
            _ => {}
        }

        true
    }

    fn scroll_at(&mut self, position: Position, direction: isize) {
        let hit = &self.state.hit;
        let (results, media, settings) = (hit.search_results, hit.media_list, hit.settings_list);

        if results.contains(position) {
            self.move_result(direction);
        } else if media.contains(position) {
            self.select_media(direction);
        } else if settings.contains(position) {
            self.update_settings_selection(direction);
        } else {
            // Content is the fallback: the Dashboard preview reads the same
            // offset as the Viewer, and a wheel over the chrome should still
            // move the text rather than do nothing.
            self.scroll_content(direction * WHEEL_LINES);
        }
    }

    fn click_at(&mut self, position: Position) {
        if position.y == self.state.hit.tab_row {
            if let Some(index) = self
                .state
                .hit
                .tabs
                .iter()
                .position(|(start, end)| position.x >= *start && position.x < *end)
            {
                self.state.current_tab = index;
                return;
            }
        }

        let hit = &self.state.hit;
        let (results, media, settings) = (hit.search_results, hit.media_list, hit.settings_list);

        if results.contains(position) {
            // The list scrolls independently of the selection once the matches
            // outgrow the pane, so the clicked row only means something when
            // read through the list's own offset.
            let row = (position.y - results.y) as usize;
            let index = self.state.result_list_state.offset() + row;
            if index < self.state.search_results.len() {
                self.state.selected_result = index;
                self.state.result_list_state.select(Some(index));
            }
        } else if media.contains(position) {
            // Each image occupies two rows: its title and its detail line.
            let index = (position.y - media.y) as usize / 2;
            if index < self.state.media.len() {
                self.state.selected_media = index;
            }
        } else if settings.contains(position) {
            let row = (position.y - settings.y) as usize;
            self.state.settings_selection = row.min(SETTINGS_LAST);
        }
    }

    pub fn tick(&mut self) {
        self.last_tick = Instant::now();
    }
}

/// Vim's movement keys, mapped onto the keys every mode already understands so
/// they work the same way everywhere.
///
/// `h`/`l` are deliberately absent: `l` toggles line numbers, and providing one
/// half of a symmetric pair would be worse than providing neither.
fn vim_alias(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::Char('j') => KeyCode::Down,
        KeyCode::Char('k') => KeyCode::Up,
        KeyCode::Char('g') => KeyCode::Home,
        KeyCode::Char('G') => KeyCode::End,
        other => other,
    }
}
