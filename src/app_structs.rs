use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

use crate::media::{GraphicsMode, MediaItem};
use crate::pdf_doc::PdfDocument;
use crate::text_analysis::{Keyword, Phrase, Readability, SearchOptions};

/// Workspace modes, in tab order.
pub const TAB_COUNT: usize = 6;
pub const TAB_DASHBOARD: usize = 0;
pub const TAB_SEARCH: usize = 1;
pub const TAB_VIEWER: usize = 2;
pub const TAB_ANALYZE: usize = 3;
pub const TAB_MEDIA: usize = 4;
pub const TAB_SETTINGS: usize = 5;

#[derive(Parser, Debug)]
#[command(name = "flerp")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A TUI for text analysis and keyword extraction", long_about = None)]
pub struct Cli {
    #[arg(help = "Path to the file to analyze")]
    pub file: Option<String>,

    #[arg(
        long,
        value_enum,
        default_value_t = GraphicsMode::Auto,
        help = "Terminal graphics protocol to use for images"
    )]
    pub graphics: GraphicsMode,

    #[arg(
        long,
        conflicts_with = "text",
        help = "Print the analysis as JSON and exit instead of starting the TUI"
    )]
    pub json: bool,

    #[arg(
        long,
        help = "Print the file's extracted plain text and exit instead of starting the TUI"
    )]
    pub text: bool,

    #[arg(
        long,
        value_name = "DIR",
        help = "Write every embedded image to DIR as PNG and exit"
    )]
    pub extract_images: Option<PathBuf>,

    #[arg(
        short,
        long,
        value_name = "QUERY",
        help = "Search query: printed as matches in headless mode, pre-filled in the TUI otherwise"
    )]
    pub search: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Open the viewer on this page (paged documents only)"
    )]
    pub page: Option<usize>,

    #[arg(short = 'i', long, help = "Match the search query case-insensitively")]
    pub ignore_case: bool,

    #[arg(short = 'e', long, help = "Treat the search query as a regular expression")]
    pub regex: bool,

    #[arg(short = 'w', long, help = "Match the search query on whole words only")]
    pub word: bool,

    #[arg(
        long,
        value_name = "N",
        default_value_t = 10,
        help = "How many keywords the headless analysis reports"
    )]
    pub keywords: usize,
}

impl Cli {
    /// True when the flags ask for output rather than a terminal session.
    ///
    /// `--search` alone is not enough: on its own it seeds the TUI's search box,
    /// and only turns into printed matches alongside one of these.
    pub fn is_headless(&self) -> bool {
        self.json || self.text || self.extract_images.is_some()
    }

    pub fn search_options(&self) -> SearchOptions {
        SearchOptions {
            case_sensitive: !self.ignore_case,
            regex_mode: self.regex,
            whole_word: self.word,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
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

/// What the next keystroke means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Keys are commands.
    Normal,
    /// Keys go into the search query.
    Search,
    /// Keys go into the line-number prompt opened with `:`.
    Goto,
}

/// Where things ended up on screen during the last draw.
///
/// Mouse events arrive as absolute terminal coordinates and know nothing about
/// the layout, so the only way to say what was clicked is to record the regions
/// while drawing them. Everything here is stale by one frame at worst, which is
/// the same frame the user was looking at when they clicked.
#[derive(Debug, Clone, Default)]
pub struct HitRegions {
    /// Horizontal span of each tab label, in tab order, plus the row they sit on.
    pub tabs: Vec<(u16, u16)>,
    pub tab_row: u16,
    /// Text area of the viewer, excluding its border.
    pub viewer: Rect,
    pub search_results: Rect,
    pub media_list: Rect,
    pub settings_list: Rect,
}

#[derive(Clone)]
pub struct AppState {
    pub file_content: String,
    pub file_name: String,
    pub search_query: String,
    pub search_results: Vec<SearchMatch>,
    pub search_error: Option<String>,
    pub keywords: Vec<Keyword>,
    pub phrases: Vec<Phrase>,
    pub readability: Readability,
    pub repeated_lines: Vec<(String, usize)>,
    pub structural_analysis: StructuralAnalysisResults,
    pub current_tab: usize,
    pub input_mode: InputMode,
    /// Digits typed at the `:` prompt, before they become a line number.
    pub goto_buffer: String,
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
    /// Page structure, present only for PDFs.
    pub document: Option<Arc<PdfDocument>>,
    /// Images extracted from the loaded file, in page order.
    pub media: Vec<MediaItem>,
    pub selected_media: usize,
    /// Rows the viewer actually has room for, measured during the last draw.
    /// Paging keys use this so a page step matches what is on screen.
    pub viewer_height: usize,
    /// Clickable regions, measured during the last draw.
    pub hit: HitRegions,
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
            phrases: Vec::new(),
            readability: Readability::default(),
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
            input_mode: InputMode::Normal,
            goto_buffer: String::new(),
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
            document: None,
            media: Vec::new(),
            selected_media: 0,
            viewer_height: 50,
            hit: HitRegions::default(),
        }
    }
}

impl AppState {
    /// True while the search query is being typed.
    pub fn search_mode(&self) -> bool {
        self.input_mode == InputMode::Search
    }

    /// True while keystrokes are going into a prompt rather than acting as
    /// commands. Every command key is gated on this.
    pub fn is_typing(&self) -> bool {
        self.input_mode != InputMode::Normal
    }

    /// Modes whose main pane is the scrollable file content.
    pub fn shows_content(&self) -> bool {
        self.current_tab == TAB_VIEWER || self.current_tab == TAB_DASHBOARD
    }

    /// 1-based page number under the top of the viewer, when the file has pages.
    pub fn current_page(&self) -> Option<usize> {
        let document = self.document.as_ref()?;
        let index = document.page_of_line(self.content_scroll);
        document.pages.get(index).map(|page| page.number)
    }

    pub fn selected_media_item(&self) -> Option<&MediaItem> {
        self.media.get(self.selected_media)
    }
}
