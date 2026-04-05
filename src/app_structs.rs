use clap::Parser;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "flerp")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A TUI for text analysis and keyword extraction", long_about = None)]
pub struct Cli {
    #[arg(help = "Path to the file to analyze")]
    pub file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructuralAnalysisResults {
    pub lines: usize,
    pub words: usize,
    pub characters: usize,
    pub stanzas: usize,
    pub empty_lines: usize,
    pub unique_words: usize,
    pub longest_line: usize,
    pub average_word_length: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    TokyoNight,
    Catppuccin,
    RosePine,
    Nord,
    Gruvbox,
    Dracula,
    Kanagawa,
    OneDark,
    Monokai,
    SolarizedDark,
    Everforest,
    AyuDark,
    Nightfox,
    Oxocarbon,
    FlexokiDark,
}

impl Theme {
    pub const ALL: [Theme; 15] = [
        Theme::TokyoNight,
        Theme::Catppuccin,
        Theme::RosePine,
        Theme::Nord,
        Theme::Gruvbox,
        Theme::Dracula,
        Theme::Kanagawa,
        Theme::OneDark,
        Theme::Monokai,
        Theme::SolarizedDark,
        Theme::Everforest,
        Theme::AyuDark,
        Theme::Nightfox,
        Theme::Oxocarbon,
        Theme::FlexokiDark,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Theme::TokyoNight => "Tokyo Night",
            Theme::Catppuccin => "Catppuccin",
            Theme::RosePine => "Rose Pine",
            Theme::Nord => "Nord",
            Theme::Gruvbox => "Gruvbox",
            Theme::Dracula => "Dracula",
            Theme::Kanagawa => "Kanagawa",
            Theme::OneDark => "One Dark",
            Theme::Monokai => "Monokai",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::Everforest => "Everforest",
            Theme::AyuDark => "Ayu Dark",
            Theme::Nightfox => "Nightfox",
            Theme::Oxocarbon => "Oxocarbon",
            Theme::FlexokiDark => "Flexoki Dark",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|theme| *theme == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL.iter().position(|theme| *theme == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line: String,
    pub match_count: usize,
}

#[derive(Clone)]
pub struct AppState {
    pub file_content: String,
    pub file_name: String,
    pub search_query: String,
    pub search_results: Vec<SearchMatch>,
    pub search_error: Option<String>,
    pub keywords: Vec<(String, usize)>,
    pub repeated_lines: Vec<(String, usize)>,
    pub structural_analysis: StructuralAnalysisResults,
    pub current_tab: usize,
    pub search_mode: bool,
    pub case_sensitive: bool,
    pub regex_mode: bool,
    pub whole_word: bool,
    pub line_numbers: bool,
    pub wrap_lines: bool,
    pub selected_result: usize,
    pub result_list_state: ListState,
    pub theme: Theme,
    pub settings_selection: usize,
    pub keyword_limit: usize,
    pub preview_line_count: usize,
    pub content_scroll: usize,
    pub status_message: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_content: String::new(),
            file_name: "No file loaded".to_string(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_error: None,
            keywords: Vec::new(),
            repeated_lines: Vec::new(),
            structural_analysis: StructuralAnalysisResults {
                lines: 0,
                words: 0,
                characters: 0,
                stanzas: 0,
                empty_lines: 0,
                unique_words: 0,
                longest_line: 0,
                average_word_length: 0.0,
            },
            current_tab: 0,
            search_mode: false,
            case_sensitive: true,
            regex_mode: false,
            whole_word: false,
            line_numbers: true,
            wrap_lines: false,
            selected_result: 0,
            result_list_state: ListState::default(),
            theme: Theme::TokyoNight,
            settings_selection: 0,
            keyword_limit: 10,
            preview_line_count: 50,
            content_scroll: 0,
            status_message: "Open a file to start searching, viewing, and analyzing text.".to_string(),
        }
    }
}
