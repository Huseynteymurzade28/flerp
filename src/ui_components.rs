use crate::app_structs::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn ui(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("✨ ", Style::default().fg(Color::Rgb(220, 180, 255))), // Pastel Purple
            Span::styled(
                "Flerp Text Analysis TUI",
                Style::default()
                    .fg(Color::Rgb(170, 200, 255)) // Pastel Blue
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("📂 File: ", Style::default().fg(Color::Rgb(150, 150, 150))), // Medium Gray
            Span::styled(
                &state.file_name,
                Style::default().fg(Color::Rgb(200, 220, 255)),
            ), // Light Pastel Blue
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(170, 200, 255))) // Pastel Blue
            .title(Span::styled(
                " ⚜️ Flerp ⚜️ ", // Changed icon for "elite" feel
                Style::default()
                    .fg(Color::Rgb(158, 210, 243)) // Pastel Pink
                    .add_modifier(Modifier::BOLD),
            )),
    )
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Tabs
    let tab_titles = vec!["📊 Overview", "🔑 Keywords", "🔎 Search", "📜 Content"];
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(180, 220, 200))) // Pastel Mint
                .title(Span::styled(
                    "🚀 Navigation 🚀",
                    Style::default()
                        .fg(Color::Rgb(180, 220, 200)) // Pastel Mint
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .style(
            Style::default()
                .fg(Color::Rgb(210, 210, 210))
                .bg(Color::Rgb(70, 70, 90)),
        ) // Light Gray on Dark Pastel Purple/Blue
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(255, 220, 180)) // Pastel Peach
                .bg(Color::Rgb(100, 120, 170)) // Medium Pastel Blue
                .add_modifier(Modifier::BOLD),
        )
        .select(state.current_tab)
        .divider(Span::styled(
            " | ",
            Style::default().fg(Color::Rgb(130, 130, 150)),
        )); // Lighter divider
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
        "▶️ Press Enter to confirm search,  Esc to cancel ◀️"
    } else {
        "🚪 'q' to quit | ⇆ Tab to switch | 🔍 '/' to search | Aa 'c' for case sensitivity"
    };

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(255, 200, 220))) // Light Pastel Pink
                .title(Span::styled(
                    "💡 Help 💡",
                    Style::default()
                        .fg(Color::Rgb(255, 200, 220)) // Light Pastel Pink
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .style(Style::default().fg(Color::Rgb(240, 240, 180))) // Pastel Yellow
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
            Span::styled("📏 Lines: ", Style::default().fg(Color::Rgb(180, 220, 255))), // Light Pastel Blue
            Span::styled(
                state.structural_analysis.lines.to_string(),
                Style::default().fg(Color::Rgb(255, 230, 200)), // Light Pastel Peach
            ),
        ]),
        Line::from(vec![
            Span::styled("📝 Words: ", Style::default().fg(Color::Rgb(180, 220, 255))), // Light Pastel Blue
            Span::styled(
                state.structural_analysis.words.to_string(),
                Style::default().fg(Color::Rgb(255, 230, 200)), // Light Pastel Peach
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "🔤 Characters: ",
                Style::default().fg(Color::Rgb(180, 220, 255)),
            ), // Light Pastel Blue
            Span::styled(
                state.structural_analysis.characters.to_string(),
                Style::default().fg(Color::Rgb(255, 230, 200)), // Light Pastel Peach
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "📑 Stanzas: ",
                Style::default().fg(Color::Rgb(180, 220, 255)),
            ), // Light Pastel Blue
            Span::styled(
                state.structural_analysis.stanzas.to_string(),
                Style::default().fg(Color::Rgb(255, 230, 200)), // Light Pastel Peach
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Aa Case Sensitive: ",
                Style::default().fg(Color::Rgb(170, 170, 170)),
            ), // Medium Gray
            Span::styled(
                if state.case_sensitive {
                    "ON ✔️"
                } else {
                    "OFF ❌"
                },
                Style::default().fg(if state.case_sensitive {
                    Color::Rgb(180, 255, 180) // Pastel Green
                } else {
                    Color::Rgb(255, 180, 180) // Pastel Red
                }),
            ),
        ]),
    ];

    let stats = Paragraph::new(stats_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(180, 255, 180))) // Pastel Green
                .title(Span::styled(
                    "📊 File Statistics 📊",
                    Style::default()
                        .fg(Color::Rgb(180, 255, 180)) // Pastel Green
                        .add_modifier(Modifier::BOLD),
                )),
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Words Progress")
                .border_style(Style::default().fg(Color::Rgb(176, 220, 160))), // Pastel Pink
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(176, 220, 160)) // Pastel Pink
                .bg(Color::Rgb(80, 80, 100)) // Darker Pastel Purple/Blue
                .add_modifier(Modifier::ITALIC),
        )
        .ratio(words_ratio)
        .label(format!("{:.0}%", words_ratio * 100.0));
    f.render_widget(words_gauge, progress_chunks[0]);

    let lines_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Lines Progress")
                .border_style(Style::default().fg(Color::Rgb(180, 220, 200))), // Pastel Mint
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(180, 220, 200)) // Pastel Mint
                .bg(Color::Rgb(80, 80, 100)) // Darker Pastel Purple/Blue
                .add_modifier(Modifier::ITALIC),
        )
        .ratio(lines_ratio)
        .label(format!("{:.0}%", lines_ratio * 100.0));
    f.render_widget(lines_gauge, progress_chunks[1]);
}

fn render_keywords(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .keywords
        .iter()
        .enumerate()
        .map(|(i, (keyword, count))| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:02}. ", i + 1),
                    Style::default().fg(Color::Rgb(150, 150, 150)), // Medium Gray
                ),
                Span::styled(
                    keyword,
                    Style::default()
                        .fg(Color::Rgb(180, 255, 180)) // Pastel Green
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ({})", count),
                    Style::default().fg(Color::Rgb(180, 220, 255)), // Light Pastel Blue
                ),
            ]))
        })
        .collect();

    let keywords_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(255, 230, 200))) // Light Pastel Peach
                .title(Span::styled(
                    "🔑 Top Keywords 🔑",
                    Style::default()
                        .fg(Color::Rgb(255, 230, 200)) // Light Pastel Peach
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(255, 220, 180)) // Pastel Peach
                .fg(Color::Rgb(60, 60, 80)) // Dark Purple/Blue
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

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
            Span::styled("🔍 Query: ", Style::default().fg(Color::Rgb(170, 200, 255))), // Pastel Blue
            Span::styled(
                &state.search_query,
                Style::default().fg(Color::Rgb(255, 220, 180)),
            ), // Pastel Peach
        ]),
        Line::from(vec![
            Span::styled(
                "🎯 Results: ",
                Style::default().fg(Color::Rgb(170, 200, 255)),
            ), // Pastel Blue
            Span::styled(
                state.search_results.len().to_string(),
                Style::default().fg(Color::Rgb(180, 255, 180)), // Pastel Green
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(220, 180, 255))) // Pastel Purple
            .title(Span::styled(
                "🔎 Search Info 🔎",
                Style::default()
                    .fg(Color::Rgb(220, 180, 255)) // Pastel Purple
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(search_info, chunks[0]);

    // Results
    if state.search_results.is_empty() {
        let no_results = Paragraph::new("🤷 No results found. Press '/' to start searching. 🤷")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(120, 120, 120))) // Dark Gray
                    .title("Search Results"),
            )
            .style(Style::default().fg(Color::Rgb(150, 150, 150))) // Medium Gray
            .alignment(Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        let items: Vec<ListItem> = state
            .search_results
            .iter()
            .enumerate()
            .map(|(i, line)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:02}. ", i + 1),
                        Style::default().fg(Color::Rgb(150, 150, 150)), // Medium Gray
                    ),
                    Span::styled(line, Style::default().fg(Color::Rgb(220, 220, 220))), // Light Gray
                ]))
            })
            .collect();

        let results_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(180, 255, 180))) // Pastel Green
                    .title(Span::styled(
                        "📜 Search Results 📜",
                        Style::default()
                            .fg(Color::Rgb(180, 255, 180)) // Pastel Green
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(180, 255, 180)) // Pastel Green
                    .fg(Color::Rgb(60, 60, 80)) // Dark Purple/Blue
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("-> ");

        f.render_stateful_widget(results_list, chunks[1], &mut state.result_list_state);
    }
}

fn render_content(f: &mut Frame, area: Rect, state: &AppState) {
    let content = if state.file_content.is_empty() {
        "🚫 No file loaded. Load a file to see its content here. 🚫".to_string()
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
                .border_style(Style::default().fg(Color::Rgb(200, 220, 255))) // Light Pastel Blue
                .title(Span::styled(
                    "📄 File Content (First 50 lines) 📄",
                    Style::default()
                        .fg(Color::Rgb(200, 220, 255)) // Light Pastel Blue
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Rgb(210, 210, 210))); // Light Gray

    f.render_widget(paragraph, area);
}

fn render_search_input(f: &mut Frame, state: &AppState) {
    let popup_area = centered_rect(60, 25, f.area());
    f.render_widget(Clear, popup_area);
    let input_text = format!("Search 🔎: {}_", state.search_query);
    let input = Paragraph::new(input_text)
        .style(
            Style::default()
                .fg(Color::Rgb(255, 220, 180)) // Pastel Peach
                .bg(Color::Rgb(60, 60, 80)), // Dark Pastel Purple/Blue
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    "✏️ Enter Search Query ✏️",
                    Style::default()
                        .fg(Color::Rgb(255, 180, 220)) // Pastel Pink
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Color::Rgb(255, 180, 220))), // Pastel Pink
        )
        .alignment(Alignment::Center);
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
