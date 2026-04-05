use crate::app_structs::{AppState, Theme};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

struct Palette {
    background: Color,
    surface: Color,
    accent: Color,
    accent_soft: Color,
    accent_alt: Color,
    text: Color,
    muted: Color,
    success: Color,
    warning: Color,
    danger: Color,
    highlight_bg: Color,
}

pub fn ui(f: &mut Frame, state: &mut AppState) {
    let palette = palette(state.theme);
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(palette.background)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, chunks[0], state, &palette);
    render_tabs(f, chunks[1], state, &palette);

    match state.current_tab {
        0 => render_dashboard(f, chunks[2], state, &palette),
        1 => render_search(f, chunks[2], state, &palette),
        2 => render_viewer(f, chunks[2], state, &palette),
        3 => render_analysis(f, chunks[2], state, &palette),
        4 => render_settings(f, chunks[2], state, &palette),
        _ => {}
    }

    render_footer(f, chunks[3], state, &palette);

    if state.search_mode {
        render_search_input(f, state, &palette);
    }
}

fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::TokyoNight => Palette {
            background: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(36, 40, 59),
            accent: Color::Rgb(122, 162, 247),
            accent_soft: Color::Rgb(187, 154, 247),
            accent_alt: Color::Rgb(158, 206, 106),
            text: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(86, 95, 137),
            success: Color::Rgb(115, 218, 202),
            warning: Color::Rgb(224, 175, 104),
            danger: Color::Rgb(247, 118, 142),
            highlight_bg: Color::Rgb(41, 46, 66),
        },
        Theme::Catppuccin => Palette {
            background: Color::Rgb(30, 30, 46),
            surface: Color::Rgb(49, 50, 68),
            accent: Color::Rgb(137, 180, 250),
            accent_soft: Color::Rgb(203, 166, 247),
            accent_alt: Color::Rgb(166, 227, 161),
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(127, 132, 156),
            success: Color::Rgb(148, 226, 213),
            warning: Color::Rgb(249, 226, 175),
            danger: Color::Rgb(243, 139, 168),
            highlight_bg: Color::Rgb(69, 71, 90),
        },
        Theme::RosePine => Palette {
            background: Color::Rgb(25, 23, 36),
            surface: Color::Rgb(38, 35, 58),
            accent: Color::Rgb(196, 167, 231),
            accent_soft: Color::Rgb(234, 154, 151),
            accent_alt: Color::Rgb(156, 207, 216),
            text: Color::Rgb(224, 222, 244),
            muted: Color::Rgb(110, 106, 134),
            success: Color::Rgb(156, 207, 216),
            warning: Color::Rgb(246, 193, 119),
            danger: Color::Rgb(235, 111, 146),
            highlight_bg: Color::Rgb(49, 44, 74),
        },
        Theme::Nord => Palette {
            background: Color::Rgb(46, 52, 64),
            surface: Color::Rgb(59, 66, 82),
            accent: Color::Rgb(129, 161, 193),
            accent_soft: Color::Rgb(180, 142, 173),
            accent_alt: Color::Rgb(163, 190, 140),
            text: Color::Rgb(236, 239, 244),
            muted: Color::Rgb(129, 161, 193),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            danger: Color::Rgb(191, 97, 106),
            highlight_bg: Color::Rgb(67, 76, 94),
        },
        Theme::Gruvbox => Palette {
            background: Color::Rgb(40, 40, 40),
            surface: Color::Rgb(60, 56, 54),
            accent: Color::Rgb(131, 165, 152),
            accent_soft: Color::Rgb(211, 134, 155),
            accent_alt: Color::Rgb(184, 187, 38),
            text: Color::Rgb(235, 219, 178),
            muted: Color::Rgb(168, 153, 132),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            danger: Color::Rgb(251, 73, 52),
            highlight_bg: Color::Rgb(80, 73, 69),
        },
        Theme::Dracula => Palette {
            background: Color::Rgb(40, 42, 54),
            surface: Color::Rgb(68, 71, 90),
            accent: Color::Rgb(139, 233, 253),
            accent_soft: Color::Rgb(189, 147, 249),
            accent_alt: Color::Rgb(80, 250, 123),
            text: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(98, 114, 164),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            danger: Color::Rgb(255, 85, 85),
            highlight_bg: Color::Rgb(68, 71, 90),
        },
        Theme::Kanagawa => Palette {
            background: Color::Rgb(31, 31, 40),
            surface: Color::Rgb(42, 42, 56),
            accent: Color::Rgb(127, 180, 202),
            accent_soft: Color::Rgb(149, 127, 184),
            accent_alt: Color::Rgb(152, 187, 108),
            text: Color::Rgb(220, 215, 186),
            muted: Color::Rgb(114, 118, 125),
            success: Color::Rgb(152, 187, 108),
            warning: Color::Rgb(224, 174, 104),
            danger: Color::Rgb(196, 86, 86),
            highlight_bg: Color::Rgb(54, 58, 79),
        },
        Theme::OneDark => Palette {
            background: Color::Rgb(40, 44, 52),
            surface: Color::Rgb(49, 54, 63),
            accent: Color::Rgb(97, 175, 239),
            accent_soft: Color::Rgb(198, 120, 221),
            accent_alt: Color::Rgb(152, 195, 121),
            text: Color::Rgb(171, 178, 191),
            muted: Color::Rgb(92, 99, 112),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            danger: Color::Rgb(224, 108, 117),
            highlight_bg: Color::Rgb(62, 68, 81),
        },
        Theme::Monokai => Palette {
            background: Color::Rgb(39, 40, 34),
            surface: Color::Rgb(54, 56, 48),
            accent: Color::Rgb(102, 217, 239),
            accent_soft: Color::Rgb(174, 129, 255),
            accent_alt: Color::Rgb(166, 226, 46),
            text: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(117, 113, 94),
            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(253, 151, 31),
            danger: Color::Rgb(249, 38, 114),
            highlight_bg: Color::Rgb(73, 72, 62),
        },
        Theme::SolarizedDark => Palette {
            background: Color::Rgb(0, 43, 54),
            surface: Color::Rgb(7, 54, 66),
            accent: Color::Rgb(38, 139, 210),
            accent_soft: Color::Rgb(108, 113, 196),
            accent_alt: Color::Rgb(133, 153, 0),
            text: Color::Rgb(147, 161, 161),
            muted: Color::Rgb(88, 110, 117),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            danger: Color::Rgb(220, 50, 47),
            highlight_bg: Color::Rgb(18, 64, 76),
        },
        Theme::Everforest => Palette {
            background: Color::Rgb(45, 53, 47),
            surface: Color::Rgb(55, 63, 56),
            accent: Color::Rgb(127, 187, 179),
            accent_soft: Color::Rgb(214, 153, 182),
            accent_alt: Color::Rgb(167, 192, 128),
            text: Color::Rgb(211, 198, 170),
            muted: Color::Rgb(133, 146, 137),
            success: Color::Rgb(167, 192, 128),
            warning: Color::Rgb(219, 188, 127),
            danger: Color::Rgb(230, 126, 128),
            highlight_bg: Color::Rgb(66, 76, 68),
        },
        Theme::AyuDark => Palette {
            background: Color::Rgb(15, 22, 32),
            surface: Color::Rgb(25, 32, 42),
            accent: Color::Rgb(57, 186, 230),
            accent_soft: Color::Rgb(255, 119, 168),
            accent_alt: Color::Rgb(186, 230, 126),
            text: Color::Rgb(227, 233, 243),
            muted: Color::Rgb(112, 128, 144),
            success: Color::Rgb(186, 230, 126),
            warning: Color::Rgb(255, 204, 102),
            danger: Color::Rgb(255, 119, 119),
            highlight_bg: Color::Rgb(36, 44, 59),
        },
        Theme::Nightfox => Palette {
            background: Color::Rgb(25, 32, 48),
            surface: Color::Rgb(41, 52, 74),
            accent: Color::Rgb(99, 124, 172),
            accent_soft: Color::Rgb(187, 154, 247),
            accent_alt: Color::Rgb(129, 161, 193),
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(131, 139, 167),
            success: Color::Rgb(129, 161, 193),
            warning: Color::Rgb(230, 175, 137),
            danger: Color::Rgb(192, 97, 110),
            highlight_bg: Color::Rgb(57, 68, 99),
        },
        Theme::Oxocarbon => Palette {
            background: Color::Rgb(22, 24, 26),
            surface: Color::Rgb(38, 41, 46),
            accent: Color::Rgb(120, 169, 255),
            accent_soft: Color::Rgb(190, 139, 255),
            accent_alt: Color::Rgb(66, 211, 146),
            text: Color::Rgb(221, 225, 230),
            muted: Color::Rgb(111, 119, 131),
            success: Color::Rgb(66, 211, 146),
            warning: Color::Rgb(255, 199, 95),
            danger: Color::Rgb(255, 123, 114),
            highlight_bg: Color::Rgb(48, 52, 58),
        },
        Theme::FlexokiDark => Palette {
            background: Color::Rgb(16, 16, 14),
            surface: Color::Rgb(28, 27, 25),
            accent: Color::Rgb(67, 133, 190),
            accent_soft: Color::Rgb(139, 126, 200),
            accent_alt: Color::Rgb(102, 128, 11),
            text: Color::Rgb(206, 205, 195),
            muted: Color::Rgb(135, 133, 128),
            success: Color::Rgb(102, 128, 11),
            warning: Color::Rgb(188, 82, 21),
            danger: Color::Rgb(175, 48, 41),
            highlight_bg: Color::Rgb(40, 39, 37),
        },
    }
}

fn render_header(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "flerp",
                Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  tui text workspace", Style::default().fg(palette.muted)),
        ]),
        Line::from(vec![
            Span::styled("File ", Style::default().fg(palette.muted)),
            Span::styled(&state.file_name, Style::default().fg(palette.text)),
            Span::styled("   Theme ", Style::default().fg(palette.muted)),
            Span::styled(state.theme.label(), Style::default().fg(palette.accent_alt)),
            Span::styled("   Matches ", Style::default().fg(palette.muted)),
            Span::styled(
                state.search_results.len().to_string(),
                Style::default().fg(palette.warning),
            ),
        ]),
    ])
    .block(panel_block("Workspace", palette.accent, palette));

    f.render_widget(header, area);
}

fn render_tabs(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let tabs = Tabs::new(vec!["Dashboard", "Search", "Viewer", "Analyze", "Settings"])
        .block(panel_block("Modes", palette.accent_soft, palette))
        .style(Style::default().fg(palette.muted))
        .highlight_style(
            Style::default()
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(palette.muted)))
        .select(state.current_tab);

    f.render_widget(tabs, area);
}

fn render_footer(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let text = if state.search_mode {
        format!("Type query | Enter apply | Esc cancel | regex {} | whole-word {}", on_off(state.regex_mode), on_off(state.whole_word))
    } else {
        format!(
            "q quit | Tab mode | / search | c case {} | r regex {} | w whole-word {} | l line nums {} | z wrap {}",
            on_off(state.case_sensitive),
            on_off(state.regex_mode),
            on_off(state.whole_word),
            on_off(state.line_numbers),
            on_off(state.wrap_lines)
        )
    };

    let footer = Paragraph::new(text)
        .style(Style::default().fg(palette.text))
        .alignment(Alignment::Center)
        .block(panel_block("Controls", palette.accent_soft, palette));

    f.render_widget(footer, area);
}

fn render_dashboard(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(area);

    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(rows[0]);

    render_stat_card(f, cards[0], "Lines", &state.structural_analysis.lines.to_string(), palette.accent, palette);
    render_stat_card(f, cards[1], "Words", &state.structural_analysis.words.to_string(), palette.accent_soft, palette);
    render_stat_card(f, cards[2], "Unique", &state.structural_analysis.unique_words.to_string(), palette.warning, palette);
    render_stat_card(f, cards[3], "Longest", &state.structural_analysis.longest_line.to_string(), palette.success, palette);

    let lower = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[1]);

    let preview = build_viewer_text(state, palette, 8, true);
    let viewer = Paragraph::new(preview)
        .wrap(Wrap { trim: false })
        .block(panel_block("Content Preview", palette.accent, palette));
    f.render_widget(viewer, lower[0]);

    let matches = selected_match_text(state, palette);
    let match_panel = Paragraph::new(matches)
        .wrap(Wrap { trim: true })
        .block(panel_block("Selected Match", palette.accent_soft, palette));
    f.render_widget(match_panel, lower[1]);
}

fn render_search(f: &mut Frame, area: Rect, state: &mut AppState, palette: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .split(area);

    let mode = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Query ", Style::default().fg(palette.muted)),
            Span::styled(if state.search_query.is_empty() { "(empty)" } else { &state.search_query }, Style::default().fg(palette.accent)),
        ]),
        Line::from(vec![
            Span::styled("Case ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.case_sensitive), Style::default().fg(palette.text)),
            Span::styled("   Regex ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.regex_mode), Style::default().fg(palette.text)),
            Span::styled("   Whole word ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.whole_word), Style::default().fg(palette.text)),
        ]),
    ])
    .block(panel_block("Search Model", palette.accent, palette));
    f.render_widget(mode, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    if let Some(error) = &state.search_error {
        let panel = Paragraph::new(error.as_str())
            .style(Style::default().fg(palette.danger))
            .block(panel_block("Regex Error", palette.danger, palette));
        f.render_widget(panel, cols[0]);
    } else {
        let items: Vec<ListItem> = if state.search_results.is_empty() {
            vec![ListItem::new(Line::from(vec![Span::styled(
                "No matches yet. Press / to search.",
                Style::default().fg(palette.muted),
            )]))]
        } else {
            state
                .search_results
                .iter()
                .map(|entry| {
                    let mut spans = vec![
                        Span::styled(format!("L{:>4} ", entry.line_number), Style::default().fg(palette.warning)),
                        Span::styled(format!("x{:>2} ", entry.match_count), Style::default().fg(palette.accent_alt)),
                    ];
                    spans.extend(highlighted_line_spans(
                        entry.line.as_str(),
                        state.search_query.as_str(),
                        state.case_sensitive,
                        state.regex_mode,
                        state.whole_word,
                        palette,
                        false,
                    ));
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        let list = List::new(items)
            .block(panel_block("Matches", palette.success, palette))
            .highlight_style(
                Style::default()
                    .fg(palette.accent_alt)
                    .bg(palette.highlight_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("-> ");

        f.render_stateful_widget(list, cols[0], &mut state.result_list_state);
    }

    let detail = Paragraph::new(selected_match_text(state, palette))
        .wrap(Wrap { trim: true })
        .block(panel_block("Selection", palette.warning, palette));
    f.render_widget(detail, cols[1]);
}

fn render_viewer(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .split(area);

    let total_lines = state.file_content.lines().count();
    let end = (state.content_scroll + state.preview_line_count).min(total_lines);
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Window ", Style::default().fg(palette.muted)),
            Span::styled(format!("{}-{} of {}", state.content_scroll.saturating_add(1), end.max(state.content_scroll.saturating_add(1)), total_lines), Style::default().fg(palette.text)),
        ]),
        Line::from(vec![
            Span::styled("Line numbers ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.line_numbers), Style::default().fg(palette.text)),
            Span::styled("   Wrap ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.wrap_lines), Style::default().fg(palette.text)),
        ]),
    ])
    .block(panel_block("Viewer", palette.accent, palette));
    f.render_widget(info, rows[0]);

    let viewer = Paragraph::new(build_viewer_text(state, palette, state.preview_line_count, false))
        .wrap(Wrap { trim: false })
        .block(panel_block("Content", palette.accent_soft, palette));
    f.render_widget(viewer, rows[1]);
}

fn render_analysis(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(8)])
        .split(cols[0]);

    let metrics = Paragraph::new(vec![
        Line::from(format!("Lines: {}", state.structural_analysis.lines)),
        Line::from(format!("Words: {}", state.structural_analysis.words)),
        Line::from(format!("Characters: {}", state.structural_analysis.characters)),
        Line::from(format!("Stanzas: {}", state.structural_analysis.stanzas)),
        Line::from(format!("Unique words: {}", state.structural_analysis.unique_words)),
        Line::from(format!("Empty lines: {}", state.structural_analysis.empty_lines)),
        Line::from(format!("Longest line: {}", state.structural_analysis.longest_line)),
        Line::from(format!("Avg word length: {:.2}", state.structural_analysis.average_word_length)),
    ])
    .style(Style::default().fg(palette.text))
    .block(panel_block("Metrics", palette.accent, palette));
    f.render_widget(metrics, left[0]);

    let repeated_items: Vec<ListItem> = if state.repeated_lines.is_empty() {
        vec![ListItem::new(Line::from("No repeated non-empty lines."))]
    } else {
        state
            .repeated_lines
            .iter()
            .map(|(line, count)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("x{:>2} ", count), Style::default().fg(palette.warning)),
                    Span::styled(line.as_str(), Style::default().fg(palette.text)),
                ]))
            })
            .collect()
    };
    let repeated = List::new(repeated_items).block(panel_block("Repeated Lines", palette.warning, palette));
    f.render_widget(repeated, left[1]);

    let keyword_items: Vec<ListItem> = if state.keywords.is_empty() {
        vec![ListItem::new(Line::from("No keywords available."))]
    } else {
        state
            .keywords
            .iter()
            .enumerate()
            .map(|(index, (keyword, count))| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:02}. ", index + 1), Style::default().fg(palette.muted)),
                    Span::styled(format!("{:<18}", keyword), Style::default().fg(palette.accent_alt).add_modifier(Modifier::BOLD)),
                    Span::styled(count.to_string(), Style::default().fg(palette.warning)),
                ]))
            })
            .collect()
    };
    let keywords = List::new(keyword_items).block(panel_block("Top Keywords", palette.success, palette));
    f.render_widget(keywords, cols[1]);
}

fn render_settings(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let settings = [
        ("Theme", state.theme.label().to_string()),
        ("Keyword limit", state.keyword_limit.to_string()),
        ("Preview lines", state.preview_line_count.to_string()),
        ("Case sensitive", on_off(state.case_sensitive).to_string()),
        ("Regex mode", on_off(state.regex_mode).to_string()),
        ("Whole word", on_off(state.whole_word).to_string()),
        ("Line numbers", on_off(state.line_numbers).to_string()),
        ("Wrap lines", on_off(state.wrap_lines).to_string()),
    ];

    let items: Vec<ListItem> = settings
        .iter()
        .enumerate()
        .map(|(index, (label, value))| {
            let style = if index == state.settings_selection {
                Style::default()
                    .fg(palette.accent_alt)
                    .bg(palette.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<16}", label), style),
                Span::styled(value.as_str(), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(panel_block("Settings", palette.accent_soft, palette));
    f.render_widget(list, cols[0]);

    let theme_lines: Vec<Line> = Theme::ALL
        .iter()
        .map(|theme| {
            let style = if *theme == state.theme {
                Style::default().fg(palette.accent_alt).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text)
            };
            Line::from(Span::styled(format!("{} {}", if *theme == state.theme { ">" } else { " " }, theme.label()), style))
        })
        .collect();

    let right = Paragraph::new(Text::from(theme_lines))
        .wrap(Wrap { trim: true })
        .block(panel_block("Theme Presets", palette.warning, palette));
    f.render_widget(right, cols[1]);
}

fn render_search_input(f: &mut Frame, state: &AppState, palette: &Palette) {
    let popup_area = centered_rect(70, 24, f.area());
    f.render_widget(Clear, popup_area);

    let input = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Search ", Style::default().fg(palette.muted)),
            Span::styled(format!("{}_", state.search_query), Style::default().fg(palette.text)),
        ]),
        Line::from(vec![
            Span::styled("Regex ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.regex_mode), Style::default().fg(palette.accent_alt)),
            Span::styled("   Whole word ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.whole_word), Style::default().fg(palette.accent_alt)),
        ]),
    ])
    .alignment(Alignment::Left)
    .style(Style::default().fg(palette.text).bg(palette.surface))
    .block(panel_block("Search Query", palette.accent, palette));

    f.render_widget(input, popup_area);
}

fn build_viewer_text(state: &AppState, palette: &Palette, line_limit: usize, compact: bool) -> Text<'static> {
    if state.file_content.is_empty() {
        return Text::from(Line::from(Span::styled(
            "No file loaded. Launch flerp with a file path.",
            Style::default().fg(palette.muted),
        )));
    }

    let start = state.content_scroll;
    let selected_line = state
        .search_results
        .get(state.selected_result)
        .map(|entry| entry.line_number);

    let lines: Vec<Line<'static>> = state
        .file_content
        .lines()
        .enumerate()
        .skip(start)
        .take(line_limit)
        .map(|(index, line)| {
            let line_number = index + 1;
            let is_selected = selected_line == Some(line_number);
            let base_style = if is_selected {
                Style::default().fg(palette.text).bg(palette.highlight_bg)
            } else {
                Style::default().fg(palette.text)
            };

            let mut spans = Vec::new();
            if state.line_numbers {
                spans.push(Span::styled(
                    format!("{:>4} | ", line_number),
                    Style::default().fg(if is_selected { palette.accent_alt } else { palette.muted }),
                ));
            }

            let display_line = if compact && line.chars().count() > 88 {
                format!("{}...", line.chars().take(88).collect::<String>())
            } else {
                line.to_string()
            };
            spans.extend(highlighted_line_spans(
                display_line.as_str(),
                state.search_query.as_str(),
                state.case_sensitive,
                state.regex_mode,
                state.whole_word,
                palette,
                is_selected,
            ));
            if spans.len() == usize::from(state.line_numbers) {
                spans.push(Span::styled(display_line, base_style));
            }
            Line::from(spans)
        })
        .collect();

    Text::from(lines)
}

fn selected_match_text(state: &AppState, palette: &Palette) -> Text<'static> {
    if let Some(error) = &state.search_error {
        return Text::from(Line::from(Span::styled(error.clone(), Style::default().fg(palette.danger))));
    }

    let Some(selected) = state.search_results.get(state.selected_result) else {
        return Text::from(vec![
            Line::from(Span::styled("No selected result.", Style::default().fg(palette.muted))),
            Line::from(Span::styled("Press / to search, then use Up/Down to move through matches.", Style::default().fg(palette.text))),
        ]);
    };

    Text::from(vec![
        Line::from(vec![
            Span::styled("Line ", Style::default().fg(palette.muted)),
            Span::styled(selected.line_number.to_string(), Style::default().fg(palette.warning)),
            Span::styled("   Count ", Style::default().fg(palette.muted)),
            Span::styled(selected.match_count.to_string(), Style::default().fg(palette.accent_alt)),
        ]),
        Line::from(""),
        Line::from(highlighted_line_spans(
            selected.line.as_str(),
            state.search_query.as_str(),
            state.case_sensitive,
            state.regex_mode,
            state.whole_word,
            palette,
            false,
        )),
        Line::from(""),
        Line::from(Span::styled("Press Enter in Search mode to jump this line into Viewer.", Style::default().fg(palette.muted))),
    ])
}

fn highlighted_line_spans(
    line: &str,
    query: &str,
    case_sensitive: bool,
    regex_mode: bool,
    whole_word: bool,
    palette: &Palette,
    selected_line: bool,
) -> Vec<Span<'static>> {
    let base_style = if selected_line {
        Style::default().fg(palette.text).bg(palette.highlight_bg)
    } else {
        Style::default().fg(palette.text)
    };

    let highlight_style = Style::default()
        .fg(palette.background)
        .bg(palette.warning)
        .add_modifier(Modifier::BOLD);

    let Some(regex) = build_search_regex(query, case_sensitive, regex_mode, whole_word) else {
        return vec![Span::styled(line.to_string(), base_style)];
    };

    let mut spans = Vec::new();
    let mut last_end = 0;

    for matched in regex.find_iter(line) {
        if matched.start() > last_end {
            spans.push(Span::styled(line[last_end..matched.start()].to_string(), base_style));
        }
        spans.push(Span::styled(line[matched.start()..matched.end()].to_string(), highlight_style));
        last_end = matched.end();
    }

    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), base_style));
    } else if last_end < line.len() {
        spans.push(Span::styled(line[last_end..].to_string(), base_style));
    }

    spans
}

fn build_search_regex(
    query: &str,
    case_sensitive: bool,
    regex_mode: bool,
    whole_word: bool,
) -> Option<regex::Regex> {
    if query.is_empty() {
        return None;
    }

    let pattern = if regex_mode {
        if whole_word {
            format!(r"\b(?:{})\b", query)
        } else {
            query.to_string()
        }
    } else {
        let escaped = regex::escape(query);
        if whole_word {
            format!(r"\b{}\b", escaped)
        } else {
            escaped
        }
    };

    regex::RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .ok()
}

fn render_stat_card(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    color: Color,
    palette: &Palette,
) {
    let card = Paragraph::new(vec![
        Line::from(Span::styled(label, Style::default().fg(palette.muted))),
        Line::from(""),
        Line::from(Span::styled(value, Style::default().fg(color).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(panel_block(label, color, palette));

    f.render_widget(card, area);
}

fn panel_block<'a>(title: &'a str, color: Color, palette: &Palette) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(palette.surface))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
}

fn on_off(value: bool) -> &'static str {
    if value {
        "On"
    } else {
        "Off"
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rect);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
