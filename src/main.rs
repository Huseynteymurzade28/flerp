use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{stdout},
    time::{Duration, Instant},
};

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

pub struct App {
    state: AppState,
    last_tick: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            last_tick: Instant::now(),
        }
    }

    pub fn load_file(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(file_path)?;
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

pub fn run_tui(file_path: Option<&str>) -> Result<(), Box<dyn Error>> {
    // Terminal kurulumu
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Dosya yükleme
    if let Some(path) = file_path {
        if let Err(e) = app.load_file(path) {
            eprintln!("Error loading file: {}", e);
        }
    }

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app.state))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !app.handle_key(key.code) {
                        break;
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }
    }

    // Terminal temizleme
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Footer
        ])
        .split(f.size());

    // Header
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Flerp Text Analysis TUI",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::Gray)),
            Span::styled(&state.file_name, Style::default().fg(Color::White)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Header"))
    .alignment(Alignment::Left);
    f.render_widget(header, chunks[0]);

    // Tabs
    let tab_titles = vec!["📊 Overview", "🔤 Keywords", "🔍 Search", "📄 Content"];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Navigation"))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .select(state.current_tab);
    f.render_widget(tabs, chunks[1]);

    // Content area
    match state.current_tab {
        0 => render_overview(f, chunks[2], state),
        1 => render_keywords(f, chunks[2], state),
        2 => render_search(f, chunks[2], state),
        3 => render_content(f, chunks[2], state),
        _ => {}
    }

    // Footer
    let footer_text = if state.search_mode {
        "Press Enter to confirm search, Esc to cancel"
    } else {
        "Press 'q' to quit, Tab to switch tabs, '/' to search, 'c' to toggle case sensitivity"
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[3]);

    // Search input overlay
    if state.search_mode {
        render_search_input(f, state);
    }
}

fn render_overview(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Stats
            Constraint::Min(5),    // Visual stats
        ])
        .split(area);

    // Statistics
    let stats_text = vec![
        Line::from(vec![
            Span::styled("📏 Lines: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                state.structural_analysis.lines.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("📝 Words: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                state.structural_analysis.words.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("🔤 Characters: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                state.structural_analysis.characters.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("📑 Stanzas: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                state.structural_analysis.stanzas.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Case Sensitive: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if state.case_sensitive { "ON" } else { "OFF" },
                Style::default().fg(if state.case_sensitive {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
        ]),
    ];

    let stats = Paragraph::new(stats_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📊 File Statistics"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(stats, chunks[0]);

    // Visual progress bars
    let max_val = state
        .structural_analysis
        .words
        .max(state.structural_analysis.lines)
        .max(100) as f64;

    let words_ratio = (state.structural_analysis.words as f64 / max_val).min(1.0);
    let lines_ratio = (state.structural_analysis.lines as f64 / max_val).min(1.0);

    let progress_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(chunks[1]);

    let words_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Words"))
        .gauge_style(Style::default().fg(Color::Blue))
        .ratio(words_ratio);
    f.render_widget(words_gauge, progress_chunks[0]);

    let lines_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Lines"))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(lines_ratio);
    f.render_widget(lines_gauge, progress_chunks[1]);
}

fn render_keywords(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .keywords
        .iter()
        .enumerate()
        .map(|(i, (keyword, count))| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Gray)),
                Span::styled(
                    keyword,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" ({})", count), Style::default().fg(Color::Blue)),
            ]))
        })
        .collect();

    let keywords_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🔤 Top Keywords"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(keywords_list, area);
}

fn render_search(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search info
            Constraint::Min(5),    // Results
        ])
        .split(area);

    // Search info
    let search_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Query: ", Style::default().fg(Color::Cyan)),
            Span::styled(&state.search_query, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Results: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                state.search_results.len().to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("🔍 Search Info"),
    );
    f.render_widget(search_info, chunks[0]);

    // Results
    if state.search_results.is_empty() {
        let no_results = Paragraph::new("No results found. Press '/' to start searching.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Results"),
            )
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        let items: Vec<ListItem> = state
            .search_results
            .iter()
            .enumerate()
            .map(|(i, line)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Gray)),
                    Span::styled(line, Style::default().fg(Color::White)),
                ]))
            })
            .collect();

        let results_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Results"),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(results_list, chunks[1], &mut state.result_list_state);
    }
}

fn render_content(f: &mut Frame, area: Rect, state: &AppState) {
    let content = if state.file_content.is_empty() {
        "No file loaded. Load a file to see its content here.".to_string()
    } else {
        // İlk 50 satırı göster
        state
            .file_content
            .lines()
            .take(50)
            .collect::<Vec<_>>()
            .join("\n")
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📄 File Content (First 50 lines)"),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn render_search_input(f: &mut Frame, state: &AppState) {
    let popup_area = centered_rect(50, 20, f.size());

    f.render_widget(Clear, popup_area);

    let input_text = format!("Search: {}_", state.search_query);
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search Query")
                .border_style(Style::default().fg(Color::Blue)),
        );
    f.render_widget(input, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Mevcut fonksiyonlar (orijinal koddan)
pub fn search<'a>(query: &str, contents: &'a str) -> Vec<String> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .map(|line| line.to_string())
        .collect()
}

pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query_lower))
        .map(|line| line.to_string())
        .collect()
}

pub fn analyze_structure(contents: &str) -> StructuralAnalysisResults {
    let lines = contents.lines().count();
    let words = contents.split_whitespace().count();
    let characters = contents.chars().count();
    let stanzas = contents.split("\n\n").count();

    StructuralAnalysisResults {
        lines,
        words,
        characters,
        stanzas,
    }
}

pub fn extract_keywords(contents: &str, top_n: usize) -> Vec<(String, usize)> {
    let mut word_counts = HashMap::new();
    for word in contents.split_whitespace() {
        let cleaned_word = word
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        if !cleaned_word.is_empty() {
            *word_counts.entry(cleaned_word).or_insert(0) += 1;
        }
    }

    let mut sorted_keywords: Vec<(String, usize)> = word_counts.into_iter().collect();
    sorted_keywords.sort_by(|a, b| b.1.cmp(&a.1));
    sorted_keywords.truncate(top_n);
    sorted_keywords
}

// Main function for testing
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let file_path = if args.len() > 1 {
        Some(args[1].as_str())
    } else {
        None
    };

    run_tui(file_path)
}
