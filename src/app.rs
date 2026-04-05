use crossterm::event::KeyCode;
use std::error::Error;
use std::time::Instant;

use crate::app_structs::AppState;
use crate::file_utils::read_file_content;
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
        let content = read_file_content(file_path)?;

        self.state.file_content = content;
        self.state.file_name = file_path.to_string();
        self.refresh_analysis();
        self.state.content_scroll = 0;
        self.update_search();
        self.state.status_message = format!("Loaded {}", self.state.file_name);
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
            .saturating_sub(self.state.preview_line_count)
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
            self.state.current_tab = 2;
            self.state.status_message = format!(
                "Jumped to line {} from search results.",
                selected.line_number
            );
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
                self.state.current_tab = 1;
            }
            KeyCode::Tab if !self.state.search_mode => {
                self.state.current_tab = (self.state.current_tab + 1) % 5;
            }
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
            KeyCode::Enter if !self.state.search_mode && self.state.current_tab == 1 => {
                self.jump_to_selected_result();
            }
            KeyCode::Backspace if self.state.search_mode => {
                self.state.search_query.pop();
                self.update_search();
            }
            KeyCode::Char(c) if self.state.search_mode => {
                self.state.search_query.push(c);
                self.update_search();
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == 1 => {
                if !self.state.search_results.is_empty() {
                    self.state.selected_result = self.state.selected_result.saturating_sub(1);
                    self.state
                        .result_list_state
                        .select(Some(self.state.selected_result));
                }
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == 1 => {
                if !self.state.search_results.is_empty() {
                    self.state.selected_result = (self.state.selected_result + 1)
                        .min(self.state.search_results.len().saturating_sub(1));
                    self.state
                        .result_list_state
                        .select(Some(self.state.selected_result));
                }
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == 2 => {
                self.scroll_content(-1);
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == 2 => {
                self.scroll_content(1);
            }
            KeyCode::PageUp if !self.state.search_mode && self.state.current_tab == 2 => {
                self.scroll_content(-(self.state.preview_line_count as isize));
            }
            KeyCode::PageDown if !self.state.search_mode && self.state.current_tab == 2 => {
                self.scroll_content(self.state.preview_line_count as isize);
            }
            KeyCode::Home if !self.state.search_mode && self.state.current_tab == 2 => {
                self.state.content_scroll = 0;
            }
            KeyCode::End if !self.state.search_mode && self.state.current_tab == 2 => {
                self.state.content_scroll = self.max_content_scroll();
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == 4 => {
                self.update_settings_selection(-1);
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == 4 => {
                self.update_settings_selection(1);
            }
            KeyCode::Left if !self.state.search_mode && self.state.current_tab == 4 => {
                self.adjust_setting(false);
            }
            KeyCode::Right if !self.state.search_mode && self.state.current_tab == 4 => {
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
