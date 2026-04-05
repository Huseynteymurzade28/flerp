use crate::app_structs::{AppState, Theme};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub keyword_limit: usize,
    pub preview_line_count: usize,
    pub case_sensitive: bool,
    pub regex_mode: bool,
    pub whole_word: bool,
    pub line_numbers: bool,
    pub wrap_lines: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::TokyoNight,
            keyword_limit: 10,
            preview_line_count: 50,
            case_sensitive: true,
            regex_mode: false,
            whole_word: false,
            line_numbers: true,
            wrap_lines: false,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };

        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };

        toml::from_str(&contents).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = config_path() else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
        fs::write(path, contents)
    }

    pub fn apply_to_state(&self, state: &mut AppState) {
        state.theme = self.theme;
        state.keyword_limit = self.keyword_limit;
        state.preview_line_count = self.preview_line_count;
        state.case_sensitive = self.case_sensitive;
        state.regex_mode = self.regex_mode;
        state.whole_word = self.whole_word;
        state.line_numbers = self.line_numbers;
        state.wrap_lines = self.wrap_lines;
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            theme: state.theme,
            keyword_limit: state.keyword_limit,
            preview_line_count: state.preview_line_count,
            case_sensitive: state.case_sensitive,
            regex_mode: state.regex_mode,
            whole_word: state.whole_word,
            line_numbers: state.line_numbers,
            wrap_lines: state.wrap_lines,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("dev", "huseyn", "flerp")?;
    Some(dirs.config_dir().join("settings.toml"))
}
