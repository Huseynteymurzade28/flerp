use crate::app_structs::{
    AppState, InputMode, Theme, TAB_ANALYZE, TAB_DASHBOARD, TAB_MEDIA, TAB_SEARCH, TAB_SETTINGS,
    TAB_VIEWER,
};
use crate::media::MediaRenderer;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Tabs, Wrap,
    },
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

pub fn ui(f: &mut Frame, state: &mut AppState, media: &mut MediaRenderer) {
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

    // Only the mode about to be drawn should own clickable regions. Without
    // this, leaving a mode would leave its panes catching clicks and wheel
    // events aimed at whatever replaced them.
    state.hit.viewer = Rect::ZERO;
    state.hit.search_results = Rect::ZERO;
    state.hit.media_list = Rect::ZERO;
    state.hit.settings_list = Rect::ZERO;

    render_header(f, chunks[0], state, &palette);
    render_tabs(f, chunks[1], state, &palette);

    match state.current_tab {
        TAB_DASHBOARD => render_dashboard(f, chunks[2], state, &palette),
        TAB_SEARCH => render_search(f, chunks[2], state, &palette),
        TAB_VIEWER => render_viewer(f, chunks[2], state, &palette),
        TAB_ANALYZE => render_analysis(f, chunks[2], state, &palette),
        TAB_MEDIA => render_media(f, chunks[2], state, media, &palette),
        TAB_SETTINGS => render_settings(f, chunks[2], state, &palette),
        _ => {}
    }

    render_footer(f, chunks[3], state, &palette);

    match state.input_mode {
        InputMode::Search => render_search_input(f, state, &palette),
        InputMode::Goto => render_goto_input(f, state, &palette),
        InputMode::Normal => {}
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

const TAB_LABELS: [&str; 6] = [
    "Dashboard", "Search", "Viewer", "Analyze", "Media", "Settings",
];

/// Padding `Tabs` puts on each side of a label, and the width of the divider
/// between two of them. Both are fixed by how the widget is configured below.
const TAB_PADDING: u16 = 1;
const TAB_DIVIDER: u16 = 3;

fn render_tabs(f: &mut Frame, area: Rect, state: &mut AppState, palette: &Palette) {
    let block = panel_block("Modes", palette.accent_soft, palette);
    let inner = block.inner(area);

    let tabs = Tabs::new(TAB_LABELS.to_vec())
        .block(block)
        .style(Style::default().fg(palette.muted))
        .highlight_style(
            Style::default()
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(palette.muted)))
        .select(state.current_tab);

    f.render_widget(tabs, area);

    // Walk the same layout the widget just drew, so a click can be turned back
    // into a tab index. Measuring here rather than guessing keeps the hitboxes
    // honest even when a label changes.
    state.hit.tab_row = inner.y;
    state.hit.tabs = TAB_LABELS
        .iter()
        .scan(inner.x, |cursor, label| {
            let width = label.chars().count() as u16 + TAB_PADDING * 2;
            let start = *cursor;
            *cursor = start + width + TAB_DIVIDER;
            // Clip to the pane: a narrow terminal truncates the strip, and a
            // hitbox past the right edge would catch clicks on nothing.
            Some((start.min(inner.right()), (start + width).min(inner.right())))
        })
        .collect();
}

fn render_footer(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    // Ordered by how much a reader needs them, because a narrow terminal only
    // gets the ones that fit. A hint clipped mid-word helps nobody.
    let hints: Vec<String> = match state.input_mode {
        InputMode::Search => vec![
            "Type query".into(),
            "Enter apply".into(),
            "Esc cancel".into(),
            format!("regex {}", on_off(state.regex_mode)),
            format!("whole-word {}", on_off(state.whole_word)),
        ],
        InputMode::Goto => vec![
            "Type a line number".into(),
            "Enter jump".into(),
            "Esc cancel".into(),
        ],
        InputMode::Normal if state.current_tab == TAB_MEDIA => vec![
            "q quit".into(),
            "Tab mode".into(),
            "j/k or click pick image".into(),
            "Enter jump to its page".into(),
        ],
        InputMode::Normal => vec![
            "q quit".into(),
            "Tab mode".into(),
            "/ search".into(),
            "n/N match".into(),
            ": line".into(),
            "[ ] page".into(),
            "j/k g/G ^d/^u scroll".into(),
            format!("c case {}", on_off(state.case_sensitive)),
            format!("r regex {}", on_off(state.regex_mode)),
            format!("w whole-word {}", on_off(state.whole_word)),
            format!("l line nums {}", on_off(state.line_numbers)),
            format!("z wrap {}", on_off(state.wrap_lines)),
        ],
    };

    let footer = Paragraph::new(fit_hints(&hints, area.width.saturating_sub(2)))
        .style(Style::default().fg(palette.text))
        .alignment(Alignment::Center)
        .block(panel_block("Controls", palette.accent_soft, palette));

    f.render_widget(footer, area);
}

/// Join as many hints as `width` can hold, dropping whole ones from the end.
fn fit_hints(hints: &[String], width: u16) -> String {
    const SEPARATOR: &str = " | ";

    let mut line = String::new();
    for hint in hints {
        let extra = hint.chars().count() + if line.is_empty() { 0 } else { SEPARATOR.len() };
        if line.chars().count() + extra > width as usize {
            break;
        }
        if !line.is_empty() {
            line.push_str(SEPARATOR);
        }
        line.push_str(hint);
    }

    line
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

    let preview = build_viewer_text(state, palette, state.preview_line_count, true);
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

        let block = panel_block("Matches", palette.success, palette);
        state.hit.search_results = block.inner(cols[0]);

        let list = List::new(items)
            .block(block)
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

fn render_viewer(f: &mut Frame, area: Rect, state: &mut AppState, palette: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .split(area);

    // Fill the pane rather than a fixed line budget, and remember the height so
    // PageUp/PageDown move by exactly one screenful.
    let visible = usize::from(rows[1].height.saturating_sub(2)).max(1);
    state.viewer_height = visible;

    let total_lines = state.file_content.lines().count();
    // Clamp here, where the pane height is known to be current. Anything that
    // scrolls before the viewer has ever been drawn -- a page jump straight from
    // another mode, or a terminal that just shrank -- would otherwise be
    // measured against a stale height.
    state.content_scroll = state
        .content_scroll
        .min(total_lines.saturating_sub(visible));

    let first = state.content_scroll + 1;
    let end = (state.content_scroll + visible).min(total_lines).max(first);

    let mut position = vec![
        Span::styled("Lines ", Style::default().fg(palette.muted)),
        Span::styled(
            format!("{first}-{end} of {total_lines}"),
            Style::default().fg(palette.text),
        ),
    ];
    if let Some(document) = &state.document {
        let index = document.page_of_line(state.content_scroll);
        let images_here = document
            .pages
            .get(index)
            .map(|page| page.image_count)
            .unwrap_or(0);

        position.push(Span::styled("   Page ", Style::default().fg(palette.muted)));
        position.push(Span::styled(
            format!("{} of {}", index + 1, document.page_count()),
            Style::default().fg(palette.accent_alt),
        ));
        position.push(Span::styled("   Images here ", Style::default().fg(palette.muted)));
        position.push(Span::styled(
            images_here.to_string(),
            Style::default().fg(palette.warning),
        ));
    }

    let info = Paragraph::new(vec![
        Line::from(position),
        Line::from(vec![
            Span::styled("Line numbers ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.line_numbers), Style::default().fg(palette.text)),
            Span::styled("   Wrap ", Style::default().fg(palette.muted)),
            Span::styled(on_off(state.wrap_lines), Style::default().fg(palette.text)),
            Span::styled("   [ ] page step", Style::default().fg(palette.muted)),
        ]),
    ])
    .block(panel_block("Viewer", palette.accent, palette));
    f.render_widget(info, rows[0]);

    let block = panel_block("Content", palette.accent_soft, palette);
    state.hit.viewer = block.inner(rows[1]);

    let content = build_viewer_text(state, palette, visible, false);
    let mut viewer = Paragraph::new(content).block(block);
    // Wrapping reflows long lines onto extra rows, which would push the tail of
    // the window off-screen; only apply it when the user asked for it.
    if state.wrap_lines {
        viewer = viewer.wrap(Wrap { trim: false });
    }
    f.render_widget(viewer, rows[1]);

    render_content_scrollbar(f, rows[1], state, palette, total_lines, visible);
}

/// Draw the viewer's position indicator, with the search matches marked on its
/// track.
///
/// The marks are the point: a scrollbar alone says where you are, but a long
/// document with a handful of matches also needs to say where they are, and
/// scrolling to find out is exactly the work this saves.
fn render_content_scrollbar(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    palette: &Palette,
    total_lines: usize,
    visible: usize,
) {
    // The track sits on the block's right border, between its corners.
    let track = area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });
    if track.height == 0 || total_lines <= visible {
        return;
    }

    let mut scrollbar_state = ScrollbarState::new(total_lines)
        .viewport_content_length(visible)
        .position(state.content_scroll);

    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_style(Style::default().fg(palette.muted))
            .thumb_style(Style::default().fg(palette.accent)),
        track,
        &mut scrollbar_state,
    );

    if state.search_results.is_empty() {
        return;
    }

    // Paint over track cells only. The thumb says where the user is looking,
    // which stays more useful than a mark that happens to fall under it.
    let column = track.right().saturating_sub(1);
    let buffer = f.buffer_mut();
    for entry in &state.search_results {
        let line = entry.line_number.saturating_sub(1);
        let row = track.y + (line * track.height as usize / total_lines.max(1)) as u16;
        if row >= track.bottom() {
            continue;
        }

        let cell = &mut buffer[(column, row)];
        if cell.symbol() != THUMB_SYMBOL {
            cell.set_symbol("▪");
            cell.set_fg(palette.warning);
        }
    }
}

/// What `Scrollbar` paints for the thumb, so match marks can avoid it.
const THUMB_SYMBOL: &str = "█";

fn render_media(
    f: &mut Frame,
    area: Rect,
    state: &mut AppState,
    media: &mut MediaRenderer,
    palette: &Palette,
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(5)])
        .split(columns[0]);

    let items: Vec<ListItem> = if state.media.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No images in this file",
            Style::default().fg(palette.muted),
        )))]
    } else {
        state
            .media
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let selected = index == state.selected_media;
                let marker = if selected { "▸ " } else { "  " };
                ListItem::new(vec![
                    Line::from(Span::styled(
                        format!("{marker}{}", item.title),
                        Style::default()
                            .fg(if selected { palette.accent_alt } else { palette.text })
                            .add_modifier(if selected {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    )),
                    Line::from(Span::styled(
                        format!("  {}", item.detail),
                        Style::default().fg(palette.muted),
                    )),
                ])
            })
            .collect()
    };

    let block = panel_block("Images", palette.accent, palette);
    state.hit.media_list = block.inner(rows[0]);
    f.render_widget(List::new(items).block(block), rows[0]);

    let mut status = vec![Line::from(vec![
        Span::styled("Renderer ", Style::default().fg(palette.muted)),
        Span::styled(
            media.backend_label(),
            Style::default().fg(if media.is_pixel_perfect() {
                palette.success
            } else {
                palette.warning
            }),
        ),
    ])];
    status.push(Line::from(Span::styled(
        if media.is_pixel_perfect() {
            "True pixel output."
        } else {
            "Terminal lacks a graphics protocol; showing an approximation."
        },
        Style::default().fg(palette.muted),
    )));
    if let Some(document) = &state.document {
        if !document.skipped.is_empty() {
            status.push(Line::from(Span::styled(
                format!("{} image(s) skipped", document.skipped.len()),
                Style::default().fg(palette.danger),
            )));
        }
    }
    if let Some(error) = media.error() {
        status.push(Line::from(Span::styled(
            error.to_string(),
            Style::default().fg(palette.danger),
        )));
    }

    f.render_widget(
        Paragraph::new(status)
            .wrap(Wrap { trim: true })
            .block(panel_block("Output", palette.accent_soft, palette)),
        rows[1],
    );

    let title = state
        .selected_media_item()
        .map(|item| item.title.clone())
        .unwrap_or_else(|| "Preview".to_string());
    let frame_block = panel_block("Preview", palette.accent_soft, palette).title(Span::styled(
        format!(" {title} "),
        Style::default().fg(palette.accent_alt),
    ));
    let inner = frame_block.inner(columns[1]);
    f.render_widget(frame_block, columns[1]);

    match state.selected_media_item() {
        Some(item) => {
            media.select(item);
            if !media.render(f, inner) {
                f.render_widget(
                    Paragraph::new(placeholder_text(state, media, palette))
                        .wrap(Wrap { trim: true }),
                    inner,
                );
            }
        }
        None => {
            media.clear();
            f.render_widget(
                Paragraph::new(placeholder_text(state, media, palette)).wrap(Wrap { trim: true }),
                inner,
            );
        }
    }
}

fn placeholder_text(state: &AppState, media: &MediaRenderer, palette: &Palette) -> Text<'static> {
    let mut lines = Vec::new();

    if let Some(error) = media.error() {
        lines.push(Line::from(Span::styled(
            format!("Image output unavailable: {error}"),
            Style::default().fg(palette.danger),
        )));
    } else if state.media.is_empty() {
        lines.push(Line::from(Span::styled(
            "This file has no embedded images to show.",
            Style::default().fg(palette.muted),
        )));
    }

    if let Some(document) = &state.document {
        for skipped in document.skipped.iter().take(12) {
            lines.push(Line::from(Span::styled(
                format!(
                    "Page {} · {}x{} · {}",
                    skipped.page, skipped.width, skipped.height, skipped.reason
                ),
                Style::default().fg(palette.warning),
            )));
        }
    }

    Text::from(lines)
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

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Percentage(50),
            Constraint::Min(6),
        ])
        .split(cols[1]);

    render_readability(f, right[0], state, palette);

    let keyword_items: Vec<ListItem> = if state.keywords.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No keywords available.",
            Style::default().fg(palette.muted),
        )))]
    } else {
        state
            .keywords
            .iter()
            .enumerate()
            .map(|(index, keyword)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:02}. ", index + 1),
                        Style::default().fg(palette.muted),
                    ),
                    Span::styled(
                        format!("{:<18}", keyword.word),
                        Style::default()
                            .fg(palette.accent_alt)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:>4}", keyword.count),
                        Style::default().fg(palette.warning),
                    ),
                    // The weight is what separates this from a frequency count,
                    // so it belongs on screen next to the count it reorders.
                    Span::styled(
                        format!("  {:>6.1}", keyword.score),
                        Style::default().fg(palette.muted),
                    ),
                ]))
            })
            .collect()
    };
    f.render_widget(
        List::new(keyword_items).block(panel_block(
            "Top Keywords  (count / weight)",
            palette.success,
            palette,
        )),
        right[1],
    );

    let phrase_items: Vec<ListItem> = if state.phrases.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No phrase repeats itself in this text.",
            Style::default().fg(palette.muted),
        )))]
    } else {
        state
            .phrases
            .iter()
            .map(|phrase| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("x{:>2} ", phrase.count),
                        Style::default().fg(palette.warning),
                    ),
                    Span::styled(phrase.text.as_str(), Style::default().fg(palette.text)),
                ]))
            })
            .collect()
    };
    f.render_widget(
        List::new(phrase_items).block(panel_block("Repeated Phrases", palette.accent_soft, palette)),
        right[2],
    );
}

fn render_readability(f: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let readability = &state.readability;
    let band_color = match readability.lix {
        _ if readability.sentences == 0 => palette.muted,
        value if value < 35.0 => palette.success,
        value if value < 45.0 => palette.accent_alt,
        value if value < 55.0 => palette.warning,
        _ => palette.danger,
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Language  ", Style::default().fg(palette.muted)),
            Span::styled(
                readability.language.label(),
                Style::default().fg(palette.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("LIX       ", Style::default().fg(palette.muted)),
            Span::styled(
                format!("{:.1}", readability.lix),
                Style::default().fg(band_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", readability.band),
                Style::default().fg(band_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Sentences ", Style::default().fg(palette.muted)),
            Span::styled(
                readability.sentences.to_string(),
                Style::default().fg(palette.text),
            ),
            Span::styled("   words each ", Style::default().fg(palette.muted)),
            Span::styled(
                format!("{:.1}", readability.words_per_sentence),
                Style::default().fg(palette.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("Long words ", Style::default().fg(palette.muted)),
            Span::styled(
                format!("{:.0}%", readability.long_word_share * 100.0),
                Style::default().fg(palette.text),
            ),
            Span::styled(" of the text", Style::default().fg(palette.muted)),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(panel_block("Readability", palette.warning, palette)),
        area,
    );
}

fn render_settings(f: &mut Frame, area: Rect, state: &mut AppState, palette: &Palette) {
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

    let block = panel_block("Settings", palette.accent_soft, palette);
    state.hit.settings_list = block.inner(cols[0]);
    f.render_widget(List::new(items).block(block), cols[0]);

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

fn render_goto_input(f: &mut Frame, state: &AppState, palette: &Palette) {
    let popup_area = centered_rect(46, 18, f.area());
    f.render_widget(Clear, popup_area);

    let total = state.file_content.lines().count();
    let input = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Line ", Style::default().fg(palette.muted)),
            Span::styled(
                format!("{}_", state.goto_buffer),
                Style::default().fg(palette.text),
            ),
        ]),
        Line::from(Span::styled(
            format!("1-{total}   Enter jump   Esc cancel"),
            Style::default().fg(palette.muted),
        )),
    ])
    .alignment(Alignment::Left)
    .style(Style::default().fg(palette.text).bg(palette.surface))
    .block(panel_block("Go To Line", palette.accent, palette));

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
