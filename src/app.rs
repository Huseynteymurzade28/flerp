use crossterm::event::KeyCode;
use std::error::Error;
use std::time::Instant;

use crate::app_structs::{AppState, TAB_COUNT, TAB_MEDIA, TAB_SEARCH, TAB_SETTINGS, TAB_VIEWER};
use crate::file_utils::load_file;
use crate::settings::AppSettings;
use crate::text_analysis::{
    analyze_structure, extract_keywords, extract_repeated_lines, search_with_options, SearchOptions,
};

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
        self.state.keywords = extract_keywords(&self.state.file_content, self.state.keyword_limit);
        self.state.repeated_lines = extract_repeated_lines(&self.state.file_content, 8);
        self.state.content_scroll = self.state.content_scroll.min(self.max_content_scroll());
    }

    fn update_settings_selection(&mut self, direction: isize) {
        let max_index = 7;
        let next = (self.state.settings_selection as isize + direction).clamp(0, max_index);
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

        // Land on the page start verbatim; the viewer clamps to the real pane
        // height when it draws, which is the only place that height is known.
        let page = &document.pages[target];
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

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') if !self.state.search_mode => return false,
            KeyCode::Esc => {
                if self.state.search_mode {
                    self.state.search_mode = false;
                } else {
                    return false;
                }
            }
            KeyCode::Char('/') if !self.state.search_mode => {
                self.state.search_mode = true;
                self.state.current_tab = TAB_SEARCH;
            }
            KeyCode::Tab if !self.state.search_mode => {
                self.state.current_tab = (self.state.current_tab + 1) % TAB_COUNT;
            }
            KeyCode::Char('[') if !self.state.search_mode => self.jump_page(-1),
            KeyCode::Char(']') if !self.state.search_mode => self.jump_page(1),
            KeyCode::Char('c') if !self.state.search_mode => {
                self.state.case_sensitive = !self.state.case_sensitive;
                self.update_search();
                self.persist_settings();
            }
            KeyCode::Char('r') if !self.state.search_mode => {
                self.state.regex_mode = !self.state.regex_mode;
                self.update_search();
                self.persist_settings();
            }
            KeyCode::Char('w') if !self.state.search_mode => {
                self.state.whole_word = !self.state.whole_word;
                self.update_search();
                self.persist_settings();
            }
            KeyCode::Char('l') if !self.state.search_mode => {
                self.state.line_numbers = !self.state.line_numbers;
                self.persist_settings();
            }
            KeyCode::Char('z') if !self.state.search_mode => {
                self.state.wrap_lines = !self.state.wrap_lines;
                self.persist_settings();
            }
            KeyCode::Enter if self.state.search_mode => {
                self.state.search_mode = false;
                self.update_search();
            }
            KeyCode::Enter if !self.state.search_mode && self.state.current_tab == TAB_SEARCH => {
                self.jump_to_selected_result();
            }
            KeyCode::Enter if !self.state.search_mode && self.state.current_tab == TAB_MEDIA => {
                self.jump_to_media_page();
            }
            KeyCode::Backspace if self.state.search_mode => {
                self.state.search_query.pop();
                self.update_search();
            }
            KeyCode::Char(c) if self.state.search_mode => {
                self.state.search_query.push(c);
                self.update_search();
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == TAB_SEARCH => {
                if !self.state.search_results.is_empty() {
                    self.state.selected_result = self.state.selected_result.saturating_sub(1);
                    self.state
                        .result_list_state
                        .select(Some(self.state.selected_result));
                }
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == TAB_SEARCH => {
                if !self.state.search_results.is_empty() {
                    self.state.selected_result = (self.state.selected_result + 1)
                        .min(self.state.search_results.len().saturating_sub(1));
                    self.state
                        .result_list_state
                        .select(Some(self.state.selected_result));
                }
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == TAB_VIEWER => {
                self.scroll_content(-1);
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == TAB_VIEWER => {
                self.scroll_content(1);
            }
            KeyCode::PageUp if !self.state.search_mode && self.state.current_tab == TAB_VIEWER => {
                self.scroll_content(-(self.state.viewer_height as isize));
            }
            KeyCode::PageDown if !self.state.search_mode && self.state.current_tab == TAB_VIEWER => {
                self.scroll_content(self.state.viewer_height as isize);
            }
            KeyCode::Home if !self.state.search_mode && self.state.current_tab == TAB_VIEWER => {
                self.state.content_scroll = 0;
            }
            KeyCode::End if !self.state.search_mode && self.state.current_tab == TAB_VIEWER => {
                self.state.content_scroll = self.max_content_scroll();
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == TAB_MEDIA => {
                self.select_media(-1);
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == TAB_MEDIA => {
                self.select_media(1);
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == TAB_SETTINGS => {
                self.update_settings_selection(-1);
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == TAB_SETTINGS => {
                self.update_settings_selection(1);
            }
            KeyCode::Left if !self.state.search_mode && self.state.current_tab == TAB_SETTINGS => {
                self.adjust_setting(false);
            }
            KeyCode::Right if !self.state.search_mode && self.state.current_tab == TAB_SETTINGS => {
                self.adjust_setting(true);
            }
            _ => {}
        }
        true
    }

    pub fn tick(&mut self) {
        self.last_tick = Instant::now();
    }
}
