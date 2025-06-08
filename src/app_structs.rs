use clap::Parser;
use ratatui::widgets::ListState;

#[derive(Parser, Debug)]
#[command(name = "flerp")]
#[command(version = "0.2.0")]
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
}

#[derive(Clone)]
pub struct AppState {
    pub file_content: String,
    pub file_name: String,
    pub search_query: String,
    pub search_results: Vec<String>,
    pub keywords: Vec<(String, usize)>,
    pub structural_analysis: StructuralAnalysisResults,
    pub current_tab: usize,
    pub search_mode: bool,
    pub case_sensitive: bool,
    pub selected_result: usize,
    pub result_list_state: ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_content: String::new(),
            file_name: "No file loaded".to_string(),
            search_query: String::new(),
            search_results: Vec::new(),
            keywords: Vec::new(),
            structural_analysis: StructuralAnalysisResults {
                lines: 0,
                words: 0,
                characters: 0,
                stanzas: 0,
            },
            current_tab: 0,
            search_mode: false,
            case_sensitive: true,
            selected_result: 0,
            result_list_state: ListState::default(),
        }
    }
}
