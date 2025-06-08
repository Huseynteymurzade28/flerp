use crossterm::event::KeyCode;
use std::error::Error;
use std::time::Instant;

use crate::app_structs::AppState;
use crate::file_utils::read_file_content;
use crate::text_analysis::{analyze_structure, extract_keywords, search, search_case_insensitive};

pub struct App {
    pub state: AppState,
    pub last_tick: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            last_tick: Instant::now(),
        }
    }

    pub fn load_file(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let content = read_file_content(file_path)?;

        self.state.file_content = content.clone();
        self.state.file_name = file_path.to_string();
        self.state.structural_analysis = analyze_structure(&content);
        self.state.keywords = extract_keywords(&content, 10);
        self.update_search();
        Ok(())
    }

    pub fn update_search(&mut self) {
        if !self.state.search_query.is_empty() {
            self.state.search_results = if self.state.case_sensitive {
                search(&self.state.search_query, &self.state.file_content)
            } else {
                search_case_insensitive(&self.state.search_query, &self.state.file_content)
            };
        } else {
            self.state.search_results.clear();
        }
        self.state.selected_result = 0;
        self.state.result_list_state.select(Some(0));
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
            }
            KeyCode::Tab if !self.state.search_mode => {
                self.state.current_tab = (self.state.current_tab + 1) % 4;
            }
            KeyCode::Char('c') if !self.state.search_mode => {
                self.state.case_sensitive = !self.state.case_sensitive;
                self.update_search();
            }
            KeyCode::Enter if self.state.search_mode => {
                self.state.search_mode = false;
                self.update_search();
            }
            KeyCode::Backspace if self.state.search_mode => {
                self.state.search_query.pop();
                self.update_search();
            }
            KeyCode::Char(c) if self.state.search_mode => {
                self.state.search_query.push(c);
                self.update_search();
            }
            KeyCode::Up if !self.state.search_mode && self.state.current_tab == 2 => {
                if !self.state.search_results.is_empty() {
                    self.state.selected_result = self.state.selected_result.saturating_sub(1);
                    self.state
                        .result_list_state
                        .select(Some(self.state.selected_result));
                }
            }
            KeyCode::Down if !self.state.search_mode && self.state.current_tab == 2 => {
                if !self.state.search_results.is_empty() {
                    self.state.selected_result = (self.state.selected_result + 1)
                        .min(self.state.search_results.len().saturating_sub(1));
                    self.state
                        .result_list_state
                        .select(Some(self.state.selected_result));
                }
            }
            _ => {}
        }
        true
    }

    pub fn tick(&mut self) {
        self.last_tick = Instant::now();
    }
}
