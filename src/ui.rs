use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect, Size},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap,
    },
    Frame,
};
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};
use tui_banner::{Banner, ColorMode, Fill};

use crate::app::{App, InputMode, LoadingState};
use crate::theme::AppTheme;

fn zebra_bg(absolute_idx: usize, sel: usize, theme: &AppTheme) -> Color {
    if absolute_idx == sel {
        theme.highlight_bg()
    } else {
        theme.panel_bg()
    }
}

fn val_to_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => {
            if let Ok(f) = s.parse::<f64>() {
                fmt_float(f)
            } else {
                s.clone()
            }
        }
        serde_json::Value::Number(n) => fmt_num(n),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn fmt_num(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(f) = n.as_f64() {
        return fmt_float(f);
    }
    n.to_string()
}

fn fmt_float(f: f64) -> String {
    if f.fract().abs() < 1e-6 {
        fmt_thousands(f as i64)
    } else {
        let int = f.trunc() as i64;
        let dec = ((f - f.trunc()).abs() * 100.0).round() as u32;
        format!("{},{:02}", fmt_thousands(int), dec)
    }
}

fn fmt_thousands(n: i64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 && c != '-' && !out.ends_with('-') {
            out.push('.');
        }
        out.push(c);
    }
    out
}

pub fn render(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Block::default().bg(app.theme.bg()), frame.area());

    let area = frame.area();

    if app.loading_state == LoadingState::Idle {
        render_welcome(frame, area, app);
        if app.show_theme_selector {
            render_theme_selector(frame, frame.area(), app);
        }
        render_toasts(frame, area, app);
        if app.show_ai_key_popup {
            render_ai_key_popup(frame, area, app);
        }
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Length(3), // Input
            Constraint::Length(4), // Player card (name + stats + form + border)
            Constraint::Min(3),    // Content
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_input(frame, chunks[1], app);
    app.input_rect = Some(chunks[1]);
    app.player_card_rect = Some(chunks[2]);
    let content_top = render_player_card(frame, chunks[2], app);
    let content_rect = Rect {
        x: chunks[3].x,
        y: content_top,
        width: chunks[3].width,
        height: area.height - content_top,
    };
    app.last_content_rect = Some(content_rect);
    render_content(frame, content_rect, app);
    if app.show_theme_selector {
        render_theme_selector(frame, frame.area(), app);
    }
    if app.show_ai_key_popup {
        render_ai_key_popup(frame, frame.area(), app);
    }
    render_toasts(frame, area, app);
}

fn render_welcome(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Min(5),     // banner + desc + input
            Constraint::Length(1),  // footer
        ])
        .split(area);

    render_header(frame, chunks[0], app);

    let middle = chunks[1];
    let banner_lines = Banner::new("FACEITUI")
        .ok()
        .map(|b| b.color_mode(ColorMode::NoColor).fill(Fill::Keep).render())
        .unwrap_or_default();
    let banner_count = banner_lines.lines().count() as u16;
    let content_h = banner_count + 4; // banner + gap(1) + input(3)
    let top_pad = if content_h < middle.height {
        (middle.height - content_h) / 2
    } else { 0u16 };

    let mid_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(banner_count),
            Constraint::Length(1),  // gap
            Constraint::Length(3),  // input
            Constraint::Min(0),
        ])
        .split(middle);

    // Banner
    let accent = app.theme.accent();
    let (ar, ag, ab) = match accent {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (128, 128, 128),
    };
    let total_rows = banner_lines.lines().count().max(1) as f32;
    let mut lines = Vec::new();
    for (row, line) in banner_lines.lines().enumerate() {
        let t = if total_rows > 1.0 { row as f32 / (total_rows - 1.0) } else { 0.0 };
        let spans: Vec<Span> = line.chars().map(|c| {
            if c == ' ' {
                Span::styled(" ", Style::default())
            } else {
                let r = ((ar as f32) * (1.0 - t) + 240.0 * t) as u8;
                let g = ((ag as f32) * (1.0 - t) + 240.0 * t) as u8;
                let b = ((ab as f32) * (1.0 - t) + 240.0 * t) as u8;
                Span::styled(c.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
            }
        }).collect();
        lines.push(Line::from(spans));
    }
    let banner_p = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(banner_p, mid_layout[1]);

    // Input - narrower, centered
    let input_w = mid_layout[3].width.min(50);
    let input_x = mid_layout[3].x + (mid_layout[3].width.saturating_sub(input_w)) / 2;
    let input_area = Rect::new(input_x, mid_layout[3].y, input_w, mid_layout[3].height);
    render_input(frame, input_area, app);
    app.input_rect = Some(input_area);

    if app.show_help {
        render_help_popup(frame, area, &app.theme);
    }
    if app.show_ai_key_popup {
        render_ai_key_popup(frame, area, app);
    }

    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled(" FACEIT", Style::default().fg(app.theme.accent()).bold()),
        Span::styled(" Stats", Style::default().fg(app.theme.fg()).bold()),
    ];
    if let Some(p) = &app.player_info {
        spans.push(Span::styled("  \u{2014} ", Style::default().fg(app.theme.muted())));
        spans.push(Span::styled(&p.nickname, Style::default().fg(app.theme.accent())));
    }
    let header = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .bg(app.theme.bg());
    frame.render_widget(header, area);

    // Version badge (right side)
    let version = Paragraph::new(Line::from(vec![
        Span::styled("v0.1  ", Style::default().fg(app.theme.muted())),
    ]))
    .alignment(Alignment::Right)
    .bg(app.theme.bg());
    frame.render_widget(version, area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let is_edit = app.input_mode == InputMode::Editing;
    let input_style = if is_edit {
        Style::default().fg(app.theme.accent())
    } else {
        Style::default().fg(app.theme.muted())
    };

    let mut title_spans = vec![Span::styled(
        if is_edit { " Search " } else { " Player " },
        Style::default().fg(app.theme.accent()).bold(),
    )];
    if is_edit && !app.search_history.is_empty() {
        let hint = match app.history_index {
            Some(i) => format!(" [{} / {}]", i + 1, app.search_history.len()),
            None => format!(" [{} hist]", app.search_history.len()),
        };
        title_spans.push(Span::styled(hint, Style::default().fg(app.theme.muted())));
    }

    let display_text = if is_edit && app.input.is_empty() {
        "nickname or player ID..."
    } else {
        &app.input
    };

    let input = Paragraph::new(display_text)
        .style(if is_edit && app.input.is_empty() {
            Style::default().fg(app.theme.muted()).italic()
        } else {
            input_style
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(input_style)
                .title(Line::from(title_spans))
                .bg(app.theme.panel_bg()),
        );

    frame.render_widget(input, area);

    if is_edit {
        frame.set_cursor_position((
            area.x + app.input.chars().count() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn render_player_card(frame: &mut Frame, area: Rect, app: &App) -> u16 {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(app.theme.accent()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(p) = &app.player_info {
        let mut line1 = vec![
            Span::styled(&p.nickname, Style::default().fg(app.theme.accent()).bold()),
        ];
        if let Some(level) = p.cs2().and_then(|c| c.skill_level) {
            line1.push(Span::styled("  Rank ", Style::default().fg(app.theme.muted())));
            line1.push(Span::styled(level.to_string(), Style::default().fg(app.theme.warn()).bold()));
        }
        if let Some(elo) = p.cs2().and_then(|c| c.faceit_elo) {
            line1.push(Span::styled("  ELO ", Style::default().fg(app.theme.muted())));
            line1.push(Span::styled(elo.to_string(), Style::default().fg(app.theme.fg())));
        }
        if let Some(c) = &p.country {
            line1.push(Span::styled("  ", Style::default().fg(app.theme.muted())));
            line1.push(Span::styled(c.to_uppercase(), Style::default().fg(app.theme.fg())));
        }

        let mut line2_spans: Vec<Span> = vec![];
        if let Some(stats) = &app.lifetime_stats {
            if let Some(lt) = &stats.lifetime {
                let get_val = |k: &str| lt.get(k).map(|v| val_to_str(v)).unwrap_or_default();
                let matches = get_val("Matches");
                let wr = get_val("Win Rate %");
                let kd = get_val("Average K/D Ratio");
                let hs = get_val("Average Headshots %");

                if !matches.is_empty() {
                    line2_spans.push(Span::styled(format!("M:{}", matches), Style::default().fg(app.theme.fg())));
                }
                if !wr.is_empty() {
                    let wr_style = get_value_style("Win Rate", &wr, &app.theme);
                    line2_spans.push(Span::styled("  ", Style::default().fg(app.theme.muted())));
                    line2_spans.push(Span::styled(format!("WR:{}", wr), wr_style));
                }
                if !kd.is_empty() {
                    let kd_style = get_value_style("K/D", &kd, &app.theme);
                    line2_spans.push(Span::styled("  ", Style::default().fg(app.theme.muted())));
                    line2_spans.push(Span::styled(format!("K/D:{}", kd), kd_style));
                }
                if !hs.is_empty() {
                    let hs_style = get_value_style("Headshots", &hs, &app.theme);
                    line2_spans.push(Span::styled("  ", Style::default().fg(app.theme.muted())));
                    line2_spans.push(Span::styled(format!("HS:{}", hs), hs_style));
                }
            }
        }

        let inner_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        let card1 = Paragraph::new(Line::from(line1)).alignment(Alignment::Center).bg(app.theme.bg());
        frame.render_widget(card1, inner_rows[0]);

        if !line2_spans.is_empty() {
            let card2 = Paragraph::new(Line::from(line2_spans)).alignment(Alignment::Center).bg(app.theme.bg());
            frame.render_widget(card2, inner_rows[1]);
        }

        // Form bar: Recent Results as colored W/L
        if let Some(stats) = &app.lifetime_stats {
            if let Some(lt) = &stats.lifetime {
                if let Some(serde_json::Value::Array(arr)) = lt.get("Recent Results") {
                    let mut form_spans = vec![
                        Span::styled("Form:", Style::default().fg(app.theme.muted())),
                    ];
                    for el in arr.iter().take(15) {
                        form_spans.push(Span::styled(" ", Style::default().fg(app.theme.bg())));
                        match el.as_str() {
                            Some("1") => form_spans.push(Span::styled("W", Style::default().fg(app.theme.success()).bold())),
                            _ => form_spans.push(Span::styled("L", Style::default().fg(app.theme.error()))),
                        }
                    }
                    let card3 = Paragraph::new(Line::from(form_spans)).alignment(Alignment::Center).bg(app.theme.bg());
                    frame.render_widget(card3, inner_rows[2]);
                }
            }
        }
    } else if app.loading_state == LoadingState::Success && !app.input.is_empty() {
        let card = Paragraph::new(Line::from(vec![
            Span::styled("Player ID: ", Style::default().fg(app.theme.muted())),
            Span::styled(&app.input, Style::default().fg(app.theme.fg())),
        ])).alignment(Alignment::Center).bg(app.theme.bg());
        frame.render_widget(card, inner);
    }
    area.y + area.height
}

fn render_content(frame: &mut Frame, area: Rect, app: &mut App) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    let content_area = vert[0];
    let footer_area = vert[1];

    if app.show_help {
        render_help_popup(frame, area, &app.theme);
        render_footer(frame, footer_area, app);
        return;
    }

    let is_success = matches!(app.loading_state, LoadingState::Success);

    if !is_success {
        match &app.loading_state {
            LoadingState::Idle => {
                let mut lines = Vec::new();
                let banner_result = Banner::new("FACEITUI")
                    .map(|b| b.color_mode(ColorMode::NoColor).fill(Fill::Keep).render());
                if let Ok(banner_text) = banner_result {
                    let accent = app.theme.accent();
                    let (ar, ag, ab) = match accent {
                        Color::Rgb(r, g, b) => (r, g, b),
                        _ => (128, 128, 128),
                    };
                    let total_rows = banner_text.lines().count().max(1) as f32;
                    for (row, line) in banner_text.lines().enumerate() {
                        let t = if total_rows > 1.0 { row as f32 / (total_rows - 1.0) } else { 0.0 };
                        let spans: Vec<Span> = line.chars().map(|c| {
                            if c == ' ' {
                                Span::styled(" ", Style::default())
                            } else {
                                let r = ((ar as f32) * (1.0 - t) + 255.0 * t) as u8;
                                let g = ((ag as f32) * (1.0 - t) + 255.0 * t) as u8;
                                let b = ((ab as f32) * (1.0 - t) + 255.0 * t) as u8;
                                Span::styled(c.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
                            }
                        }).collect();
                        lines.push(Line::from(spans));
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("FACEITUI", Style::default().fg(app.theme.accent()).bold()),
                    ]));
                }
                let msg = Paragraph::new(lines).alignment(Alignment::Center);
                frame.render_widget(msg, content_area);
            }
            LoadingState::Loading => {
                let spinner = app.spinner();
                let popup = centered_rect(35, 18, content_area);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.accent()))
                    .bg(app.theme.panel_bg())
                    .title(Line::from(vec![Span::styled(" Loading ", Style::default().fg(app.theme.accent()).bold())]));
                frame.render_widget(block.clone(), popup);
                let inner = block.inner(popup).inner(Margin { horizontal: 2, vertical: 1 });
                let msg = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(spinner, Style::default().fg(app.theme.accent()).add_modifier(Modifier::BOLD)),
                        Span::styled("  Fetching player data...", Style::default().fg(app.theme.fg())),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled("This may take a moment", Style::default().fg(app.theme.muted()))]),
                ]).bg(app.theme.panel_bg()).alignment(Alignment::Center);
                frame.render_widget(msg, inner);
            }
            LoadingState::Error(err) => {
                let err_msg = clean_error_msg(err);
                let popup = centered_rect(55, 30, content_area);
                frame.render_widget(Clear, popup);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.error()))
                    .bg(app.theme.panel_bg())
                    .title(Line::from(vec![Span::styled(" Error ", Style::default().fg(app.theme.error()).bold())]));
                frame.render_widget(block.clone(), popup);
                let inner = block.inner(popup).inner(Margin { horizontal: 2, vertical: 1 });
                let lines = vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(&err_msg, Style::default().fg(app.theme.fg()))]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("i", Style::default().fg(app.theme.accent())),
                        Span::styled(" search again  ", Style::default().fg(app.theme.muted())),
                        Span::styled("r", Style::default().fg(app.theme.accent())),
                        Span::styled(" retry  ", Style::default().fg(app.theme.muted())),
                        Span::styled("q", Style::default().fg(app.theme.accent())),
                        Span::styled(" quit", Style::default().fg(app.theme.muted())),
                    ]),
                ];
                let msg = Paragraph::new(lines).bg(app.theme.panel_bg()).alignment(Alignment::Center);
                frame.render_widget(msg, inner);
            }
            _ => {}
        }
    }

    if is_success {
        render_tabs(frame, content_area, app);
    }
    render_footer(frame, footer_area, app);
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &mut App) {
    let lt_count = app.lifetime_stats.as_ref().and_then(|s| s.lifetime.as_ref()).map(|m| m.len()).unwrap_or(0);
    let mh_count = app.match_history.as_ref().map(|v| v.len()).unwrap_or(0);
    let map_count = app.lifetime_stats.as_ref().and_then(|s| s.segments.as_ref()).map(|v| v.len()).unwrap_or(0);

    let titles = vec![
        format!(" Lifetime ({}) ", lt_count),
        format!(" Matches ({}) ", mh_count),
        format!(" Maps ({}) ", map_count),
    ];
    let tabs = Tabs::new(titles)
        .select(app.selected_tab)
        .style(Style::default().fg(app.theme.muted()).bg(app.theme.bg()))
        .highlight_style(
            Style::default()
                .fg(app.theme.accent())
                .bg(app.theme.highlight_bg())
                .add_modifier(Modifier::BOLD)
        )
        .divider(Span::styled(" \u{2502} ", Style::default().fg(app.theme.muted())));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    frame.render_widget(tabs, inner[0]);

    match app.selected_tab {
        0 => render_lifetime(frame, inner[1], app),
        1 => render_matches(frame, inner[1], app),
        2 => render_maps(frame, inner[1], app),
        _ => {}
    }

    if app.show_match_popup {
        render_match_popup(frame, area, app);
    }

    if app.show_map_popup {
        render_map_popup(frame, area, app);
    }

    if app.ai_loading {
        if !app.ai_partial.is_empty() {
            render_ai_popup(frame, area, &app.ai_partial, &mut app.ai_sv, true, &app.theme);
        } else {
            let popup = centered_rect(38, 12, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent()))
                .bg(app.theme.panel_bg())
                .title(Line::from(vec![Span::styled(" AI ", Style::default().fg(app.theme.accent()).bold())]));
            frame.render_widget(Clear, popup);
            frame.render_widget(block.clone(), popup);
            let block_inner = block.inner(popup).inner(Margin { horizontal: 2, vertical: 1 });
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(app.spinner(), Style::default().fg(app.theme.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled("  AI Thinking...", Style::default().fg(app.theme.fg())),
                ]),
                Line::from(""),
            ];
            let msg = Paragraph::new(lines).bg(app.theme.panel_bg()).alignment(Alignment::Center);
            frame.render_widget(msg, block_inner);
        }
    } else if let Some(response) = &app.ai_response {
        render_ai_popup(frame, area, response, &mut app.ai_sv, false, &app.theme);
    }
    if app.show_ai_key_popup {
        render_ai_key_popup(frame, area, app);
    }
}

fn render_lifetime(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(stats) = &app.lifetime_stats {
        if let Some(lifetime) = &stats.lifetime {
            let mut items: Vec<(&String, &serde_json::Value)> = lifetime.iter().collect();
            items.sort_by(|a, b| a.0.cmp(b.0));

            let total = items.len();
            let sel = app.selected_row;

            let rows: Vec<Row> = items
                .iter()
                .enumerate()
                .map(|(absolute_idx, (k, v))| {
                    let is_selected = absolute_idx == sel;
                    let bg = zebra_bg(absolute_idx, sel, &app.theme);

                    let label_style = if is_selected {
                        Style::default().fg(app.theme.fg()).bold().bg(bg)
                    } else {
                        Style::default().fg(app.theme.fg()).bg(bg)
                    };

                    if *k == "Recent Results" {
                        if let serde_json::Value::Array(arr) = v {
                            let spans: Vec<Span> = arr.iter().enumerate().flat_map(|(i, el)| {
                                let (ch, color) = match el.as_str() {
                                    Some("1") => ("W", app.theme.success()),
                                    _ => ("L", app.theme.error()),
                                };
                                let mut s = vec![];
                                if i > 0 {
                                    s.push(Span::styled(" ", Style::default().bg(bg)));
                                }
                                s.push(Span::styled(ch, Style::default().fg(color).bold().bg(bg)));
                                s
                            }).collect();
                            Row::new(vec![
                                Cell::from(k.as_str()).style(label_style),
                                Cell::from(Line::from(spans)),
                            ]).bg(bg)
                        } else {
                            let val_str = val_to_str(v);
                            Row::new(vec![
                                Cell::from(k.as_str()).style(label_style),
                                Cell::from(val_str).style(Style::default().fg(app.theme.fg()).bg(bg)),
                            ]).bg(bg)
                        }
                    } else {
                        let val_str = val_to_str(v);
                        let display_val = if is_timestamp_key(k) {
                            fmt_timestamp(&val_str).unwrap_or(val_str)
                        } else {
                            val_str
                        };
                        let value_style = get_value_style(k, &display_val, &app.theme).bg(bg);
                        Row::new(vec![
                            Cell::from(k.as_str()).style(label_style),
                            Cell::from(display_val).style(value_style),
                        ]).bg(bg)
                    }
                })
                .collect();

            let header = Row::new(vec!["Stat", "Value"])
                .style(Style::default().fg(app.theme.accent()).bold().bg(app.theme.panel_bg()));

            let content_height = total as u16 + 1;
            let table = Table::new(rows, &[Constraint::Percentage(50), Constraint::Percentage(50)])
                .header(header)
                .column_spacing(1);

            let mut sv = ScrollView::new(Size::new(inner.width, content_height))
                .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
                .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
            sv.render_widget(table, Rect::new(0, 0, inner.width, content_height));
            frame.render_stateful_widget(sv, inner, &mut app.lifetime_sv);
        } else {
            let msg = Paragraph::new("No lifetime stats.")
                .style(Style::default().fg(app.theme.muted()))
                .alignment(Alignment::Center)
                .bg(app.theme.panel_bg());
            frame.render_widget(msg, inner);
        }
    } else {
        let msg = Paragraph::new("Lifetime unavailable.")
            .style(Style::default().fg(app.theme.muted()))
            .alignment(Alignment::Center)
            .bg(app.theme.panel_bg());
        frame.render_widget(msg, inner);
    }
}

fn render_matches(frame: &mut Frame, area: Rect, app: &mut App) {
    let page_label = if app.match_history.is_some() {
        format!(" Matches (p.{}) ", app.current_page + 1)
    } else {
        " Matches ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .title(Line::from(Span::styled(page_label, Style::default().fg(app.theme.accent()).bold())))
        .bg(app.theme.panel_bg());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(history) = &app.match_history {
        if history.is_empty() {
            let msg = Paragraph::new("No match history.")
                .style(Style::default().fg(app.theme.muted()))
                .alignment(Alignment::Center)
                .bg(app.theme.panel_bg());
            frame.render_widget(msg, inner);
            return;
        }

        let total = history.len();
        let sel = app.selected_row;

        let rows: Vec<Row> = history
            .iter()
            .enumerate()
            .map(|(absolute_idx, s)| {
                let is_selected = absolute_idx == sel;
                let bg = zebra_bg(absolute_idx, sel, &app.theme);

                let map = s.get("Map").cloned().unwrap_or_default();
                let score = s.get("Score").cloned().unwrap_or_default();
                let result = s.get("Result").cloned().unwrap_or_default();
                let kd = s.get("K/D Ratio").cloned().unwrap_or_default();
                let adr = s.get("ADR").cloned().unwrap_or_default();
                let kills = s.get("Kills").cloned().unwrap_or_default();
                let deaths = s.get("Deaths").cloned().unwrap_or_default();
                let hs = s.get("Headshots %").or_else(|| s.get("Headshots")).cloned().unwrap_or_default();

                let result_style = match result.as_str() {
                    "1" => Style::default().fg(app.theme.success()).bold().bg(bg),
                    "0" => Style::default().fg(app.theme.error()).bg(bg),
                    _ => Style::default().fg(app.theme.muted()).bg(bg),
                };
                let result_label = match result.as_str() {
                    "1" => "W",
                    "0" => "L",
                    _ => "?",
                };

                let fg_bold = Style::default().fg(app.theme.fg()).bold().bg(bg);
                let fg_normal = Style::default().fg(app.theme.fg()).bg(bg);

                let kd_display = if kills.is_empty() || deaths.is_empty() {
                    kd
                } else {
                    format!("{}/{}", kills, deaths)
                };

                Row::new(vec![
                    Cell::from(map).style(if is_selected { fg_bold } else { fg_normal }),
                    Cell::from(score).style(Style::default().fg(app.theme.muted()).bg(bg)),
                    Cell::from(result_label).style(result_style),
                    Cell::from(kd_display).style(Style::default().fg(app.theme.accent()).bg(bg)),
                    Cell::from(adr).style(fg_normal),
                    Cell::from(hs).style(Style::default().fg(app.theme.warn()).bg(bg)),
                ])
                .bg(bg)
            })
            .collect();

        let header = Row::new(vec!["Map", "Score", "R", "K/D", "ADR", "HS%"])
            .style(Style::default().fg(app.theme.accent()).bold().bg(app.theme.panel_bg()));

        let content_height = total as u16 + 1;
        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(22),
                Constraint::Percentage(14),
                Constraint::Percentage(5),
                Constraint::Percentage(14),
                Constraint::Percentage(12),
                Constraint::Percentage(33),
            ],
        )
        .header(header)
        .column_spacing(1);

        let mut sv = ScrollView::new(Size::new(inner.width, content_height.max(1)))
            .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
            .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
        sv.render_widget(table, Rect::new(0, 0, inner.width, content_height));
        frame.render_stateful_widget(sv, inner, &mut app.matches_sv);
    } else {
        let msg = Paragraph::new("Matches unavailable.")
            .style(Style::default().fg(app.theme.muted()))
            .alignment(Alignment::Center)
            .bg(app.theme.panel_bg());
        frame.render_widget(msg, inner);
    }
}

fn render_maps(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let map_segments: Vec<&crate::api::LifetimeSegment> = app
        .lifetime_stats
        .as_ref()
        .and_then(|s| s.segments.as_ref())
        .map(|segs| segs.iter().filter(|s| s.type_field.as_deref() == Some("Map")).collect())
        .unwrap_or_default();

    if map_segments.is_empty() {
        let msg = Paragraph::new("No map stats available.")
            .style(Style::default().fg(app.theme.muted()))
            .alignment(Alignment::Center)
            .bg(app.theme.panel_bg());
        frame.render_widget(msg, inner);
        return;
    }

    let total = map_segments.len();
    let sel = app.selected_row;

            let rows: Vec<Row> = map_segments
                .iter()
                .enumerate()
                .map(|(absolute_idx, seg)| {
                    let is_selected = absolute_idx == sel;
                    let bg = zebra_bg(absolute_idx, sel, &app.theme);

                    let label = seg.label.as_deref().unwrap_or("?");
                    let stats = seg.stats.as_ref();
                    let get = |k: &str| stats.and_then(|m| m.get(k)).map(|v| val_to_str(v)).unwrap_or_default();

                    let wins = get("Wins");
                    let total_matches = get("Matches");
                    let wr = get("Win Rate %");
                    let wins_num: u32 = wins.parse().unwrap_or(0);
                    let total_num: u32 = total_matches.parse().unwrap_or(0);
                    let losses = stats
                        .and_then(|m| m.get("Losses"))
                        .map(|v| val_to_str(v))
                        .unwrap_or_else(|| total_num.saturating_sub(wins_num).to_string());

                    let label_style = if is_selected {
                        Style::default().fg(app.theme.fg()).bold().bg(bg)
                    } else {
                        Style::default().fg(app.theme.fg()).bg(bg)
                    };

                    Row::new(vec![
                        Cell::from(label).style(label_style),
                        Cell::from(total_matches).style(Style::default().fg(app.theme.muted()).bg(bg)),
                        Cell::from(wins).style(Style::default().fg(app.theme.success()).bg(bg)),
                        Cell::from(losses).style(Style::default().fg(app.theme.error()).bg(bg)),
                        Cell::from(wr.clone()).style(get_value_style("Win Rate", &wr, &app.theme).bg(bg)),
                    ])
                    .bg(bg)
                })
                .collect();

    let header = Row::new(vec!["Map", "M", "W", "L", "Win%"])
        .style(Style::default().fg(app.theme.accent()).bold().bg(app.theme.panel_bg()));

    let content_height = total as u16 + 1;
    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(35),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .column_spacing(1);

    let mut sv = ScrollView::new(Size::new(inner.width, content_height.max(1)))
        .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
    sv.render_widget(table, Rect::new(0, 0, inner.width, content_height));
    frame.render_stateful_widget(sv, inner, &mut app.maps_sv);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hints = match app.input_mode {
        InputMode::Normal => {
            if app.loading_state == LoadingState::Idle {
                vec![
                    Span::styled("i", Style::default().fg(app.theme.accent())),
                    Span::styled(" edit ", Style::default().fg(app.theme.muted())),
                    Span::styled("t", Style::default().fg(app.theme.accent())),
                    Span::styled(" themes ", Style::default().fg(app.theme.muted())),
                    Span::styled("?", Style::default().fg(app.theme.accent())),
                    Span::styled(" help ", Style::default().fg(app.theme.muted())),
                    Span::styled("q", Style::default().fg(app.theme.accent())),
                    Span::styled(" quit", Style::default().fg(app.theme.muted())),
                ]
            } else {
                let mut hints = vec![
                    Span::styled("Esc", Style::default().fg(app.theme.accent())),
                    Span::styled(" home ", Style::default().fg(app.theme.muted())),
                    Span::styled("i", Style::default().fg(app.theme.accent())),
                    Span::styled(" edit ", Style::default().fg(app.theme.muted())),
                    Span::styled("r", Style::default().fg(app.theme.accent())),
                    Span::styled(" refr ", Style::default().fg(app.theme.muted())),
                    Span::styled("Tab", Style::default().fg(app.theme.accent())),
                    Span::styled(" tabs ", Style::default().fg(app.theme.muted())),
                    Span::styled("a", Style::default().fg(app.theme.accent())),
                    Span::styled(" AI ", Style::default().fg(app.theme.muted())),
                    Span::styled("e", Style::default().fg(app.theme.accent())),
                    Span::styled(" export ", Style::default().fg(app.theme.muted())),
                    Span::styled("t", Style::default().fg(app.theme.accent())),
                    Span::styled(" themes ", Style::default().fg(app.theme.muted())),
                    Span::styled("?", Style::default().fg(app.theme.accent())),
                    Span::styled(" help ", Style::default().fg(app.theme.muted())),
                    Span::styled("q", Style::default().fg(app.theme.accent())),
                    Span::styled(" quit", Style::default().fg(app.theme.muted())),
                ];
                if app.selected_tab == 1 && app.match_history.is_some() {
                    hints.push(Span::styled("  n/p", Style::default().fg(app.theme.accent())));
                    hints.push(Span::styled(" page ", Style::default().fg(app.theme.muted())));
                    hints.push(Span::styled("Ent", Style::default().fg(app.theme.accent())));
                    hints.push(Span::styled(" detail", Style::default().fg(app.theme.muted())));
                }
                hints
            }
        }
        InputMode::Editing => vec![
            Span::styled("Enter", Style::default().fg(app.theme.accent())),
            Span::styled(" submit ", Style::default().fg(app.theme.muted())),
            Span::styled("Esc", Style::default().fg(app.theme.accent())),
            Span::styled(" cancel", Style::default().fg(app.theme.muted())),
        ],
    };

    let footer = Paragraph::new(Line::from(hints))
        .alignment(Alignment::Center)
        .bg(app.theme.bg());
    frame.render_widget(footer, area);
}

fn render_toasts(frame: &mut Frame, area: Rect, app: &App) {
    if app.toasts.is_empty() {
        return;
    }
    let toast_height = app.toasts.len().min(2) as u16 + 2;
    let toast_width = 38u16.min(area.width.saturating_sub(2));
    let toast_area = Rect {
        x: area.x + area.width - toast_width - 1,
        y: area.y + area.height.saturating_sub(toast_height + 1),
        width: toast_width,
        height: toast_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg());
    frame.render_widget(Clear, toast_area);
    frame.render_widget(block.clone(), toast_area);

    let inner = block.inner(toast_area);
    let lines: Vec<Line> = app
        .toasts
        .iter()
        .rev()
        .take(2)
        .map(|t| {
            let style = if t.is_error {
                Style::default().fg(app.theme.error()).bold()
            } else {
                Style::default().fg(app.theme.success()).bold()
            };
            Line::from(vec![
                Span::styled(if t.is_error { "x " } else { "v " }, style),
                Span::styled(&t.message, Style::default().fg(app.theme.fg())),
            ])
        })
        .collect();

    let toast = Paragraph::new(lines).bg(app.theme.panel_bg()).wrap(Wrap { trim: true });
    frame.render_widget(toast, inner);
}

fn render_ai_popup(frame: &mut Frame, area: Rect, response: &str, state: &mut ScrollViewState, streaming: bool, theme: &AppTheme) {
    let popup_w = area.width * 60 / 100;
    let inner_w = popup_w.saturating_sub(4); // borders + margins
    let chars_per_line = inner_w.saturating_sub(2) as usize;
    let line_count: usize = response.lines()
        .map(|l| (l.len() + chars_per_line) / chars_per_line.max(1))
        .sum();
    let content_height = (line_count + 2) as u16;
    let needed = (content_height + 4).clamp(10, area.height.saturating_sub(4));
    let pct_y = (needed * 100 / area.height.max(1)).clamp(20, 70) as u16;

    let popup_area = centered_rect(60, pct_y, area);
    frame.render_widget(Clear, popup_area);
    let title = if streaming {
        Line::from(vec![Span::styled(" AI Writing... ", Style::default().fg(theme.accent()).bold())])
    } else {
        Line::from(vec![Span::styled(" AI Analysis ", Style::default().fg(theme.accent()).bold())])
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .bg(theme.panel_bg())
        .title(title);
    frame.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area).inner(Margin { horizontal: 2, vertical: 1 });

    let msg = Paragraph::new(response)
        .style(Style::default().fg(theme.fg()))
        .bg(theme.panel_bg())
        .wrap(Wrap { trim: true });

    let mut sv = ScrollView::new(Size::new(inner.width, content_height.max(1)))
        .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
    sv.render_widget(msg, Rect::new(0, 0, inner.width, content_height));
    frame.render_stateful_widget(sv, inner, state);
}

fn render_match_popup(frame: &mut Frame, area: Rect, app: &mut App) {
    let history = match &app.match_history {
        Some(h) => h,
        None => return,
    };
    let match_data = match history.get(app.selected_row) {
        Some(m) => m,
        None => return,
    };

    let popup_area = centered_rect(50, 70, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg())
        .title(Line::from(vec![
            Span::styled(" Match Details ", Style::default().fg(app.theme.accent()).bold()),
        ]));
    frame.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let mut items: Vec<(&String, &String)> = match_data.iter().collect();
    items.sort_by(|a, b| match_key_order(a.0).cmp(&match_key_order(b.0)));
    let total = items.len();

    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (k, v))| {
            let bg = zebra_bg(idx, usize::MAX, &app.theme);
            let display_val = if is_timestamp_key(k) {
                fmt_timestamp(v).unwrap_or_else(|| v.to_string())
            } else {
                v.to_string()
            };
            let v_style = get_value_style(k, &display_val, &app.theme).bg(bg);
            Row::new(vec![
                Cell::from(k.as_str()).style(Style::default().fg(app.theme.muted()).bg(bg)),
                Cell::from(display_val).style(v_style),
            ])
            .bg(bg)
        })
        .collect();

    let content_height = total as u16;
    let table = Table::new(rows, &[Constraint::Percentage(40), Constraint::Percentage(60)])
        .column_spacing(1);

    let mut sv = ScrollView::new(Size::new(inner.width, content_height.max(1)))
        .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
    sv.render_widget(table, Rect::new(0, 0, inner.width, content_height));
    frame.render_stateful_widget(sv, inner, &mut app.match_popup_sv);
}

fn render_map_popup(frame: &mut Frame, area: Rect, app: &mut App) {
    let map_segments: Vec<&crate::api::LifetimeSegment> = app
        .lifetime_stats
        .as_ref()
        .and_then(|s| s.segments.as_ref())
        .map(|segs| segs.iter().filter(|s| s.type_field.as_deref() == Some("Map")).collect())
        .unwrap_or_default();

    let seg = match map_segments.get(app.selected_row) {
        Some(s) => s,
        None => return,
    };

    let label = seg.label.as_deref().unwrap_or("?");
    let stats = match seg.stats.as_ref() {
        Some(s) => s,
        None => return,
    };

    let popup_area = centered_rect(50, 70, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg())
        .title(Line::from(vec![
            Span::styled(format!(" {} Details ", label), Style::default().fg(app.theme.accent()).bold()),
        ]));
    frame.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let mut items: Vec<(&String, &serde_json::Value)> = stats.iter().collect();
    items.sort_by(|a, b| match_key_order(a.0).cmp(&match_key_order(b.0)));
    let total = items.len();

    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (k, v))| {
            let bg = zebra_bg(idx, usize::MAX, &app.theme);
            let val_str = val_to_str(v);
            let display_val = if is_timestamp_key(k) {
                fmt_timestamp(&val_str).unwrap_or(val_str)
            } else {
                val_str
            };
            let v_style = get_value_style(k, &display_val, &app.theme).bg(bg);
            Row::new(vec![
                Cell::from(k.as_str()).style(Style::default().fg(app.theme.muted()).bg(bg)),
                Cell::from(display_val).style(v_style),
            ])
            .bg(bg)
        })
        .collect();

    let content_height = total as u16;
    let table = Table::new(rows, &[Constraint::Percentage(45), Constraint::Percentage(55)])
        .column_spacing(1);

    let mut sv = ScrollView::new(Size::new(inner.width, content_height.max(1)))
        .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
    sv.render_widget(table, Rect::new(0, 0, inner.width, content_height));
    frame.render_stateful_widget(sv, inner, &mut app.map_popup_sv);
}

fn render_help_popup(frame: &mut Frame, area: Rect, theme: &AppTheme) {
    let popup_area = centered_rect(55, 65, area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent()))
            .bg(theme.panel_bg())
            .title(Line::from(vec![Span::styled(" Help ", Style::default().fg(theme.accent()).bold())])),
        popup_area,
    );

    let inner = popup_area.inner(Margin { horizontal: 2, vertical: 1 });
    let lines = vec![
        Line::from(vec![Span::styled("Navigation", Style::default().fg(theme.accent()).bold())]),
        Line::from(""),
        Line::from(vec![Span::styled("Tab", Style::default().fg(theme.accent())), Span::styled("      Switch Lifetime / Matches / Maps", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("↑/↓", Style::default().fg(theme.accent())), Span::styled("      Select row", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("PgUp/Dn", Style::default().fg(theme.accent())), Span::styled("  Page scroll", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("n/p", Style::default().fg(theme.accent())), Span::styled("      Prev/next match page", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("Scroll", Style::default().fg(theme.accent())), Span::styled("    Mouse wheel scroll", Style::default().fg(theme.fg()))]),
        Line::from(""),
        Line::from(vec![Span::styled("Search", Style::default().fg(theme.accent()).bold())]),
        Line::from(""),
        Line::from(vec![Span::styled("i", Style::default().fg(theme.accent())), Span::styled("        Enter search mode", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("Enter", Style::default().fg(theme.accent())), Span::styled("    Submit search", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("Esc", Style::default().fg(theme.accent())), Span::styled("      Cancel", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("↑/↓", Style::default().fg(theme.accent())), Span::styled("      Browse history (edit)", Style::default().fg(theme.fg()))]),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", Style::default().fg(theme.accent()).bold())]),
        Line::from(""),
        Line::from(vec![Span::styled("r", Style::default().fg(theme.accent())), Span::styled("        Refresh", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("c", Style::default().fg(theme.accent())), Span::styled("        Copy player ID", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("e", Style::default().fg(theme.accent())), Span::styled("        Export current tab to JSON", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("Enter", Style::default().fg(theme.accent())), Span::styled("    Match detail popup", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("?", Style::default().fg(theme.accent())), Span::styled("        Toggle help", Style::default().fg(theme.fg()))]),
        Line::from(vec![Span::styled("q", Style::default().fg(theme.accent())), Span::styled("        Quit", Style::default().fg(theme.fg()))]),
    ];

    let help = Paragraph::new(lines)
        .style(Style::default().fg(theme.fg()))
        .wrap(Wrap { trim: true })
        .bg(theme.panel_bg());
    frame.render_widget(help, inner);
}

fn render_ai_key_popup(frame: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(50, 30, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg())
        .title(Line::from(vec![
            Span::styled(" AI API Key ", Style::default().fg(app.theme.accent()).bold()),
        ]));
    frame.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area).inner(Margin { horizontal: 2, vertical: 1 });

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Enter your OpenCode AI API key:", Style::default().fg(app.theme.fg())),
    ])).bg(app.theme.panel_bg());
    frame.render_widget(hint, inner_chunks[0]);

    let display_text = if app.ai_key_input.is_empty() {
        "sk-..."
    } else {
        &app.ai_key_input
    };
    let input = Paragraph::new(display_text)
        .style(if app.ai_key_input.is_empty() {
            Style::default().fg(app.theme.muted()).italic()
        } else {
            Style::default().fg(app.theme.accent())
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent()))
                .bg(app.theme.bg()),
        );
    frame.render_widget(input, inner_chunks[1]);

    let footer_hint = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(app.theme.accent())),
        Span::styled(" submit  ", Style::default().fg(app.theme.muted())),
        Span::styled("Esc", Style::default().fg(app.theme.accent())),
        Span::styled(" cancel", Style::default().fg(app.theme.muted())),
    ])).alignment(Alignment::Center).bg(app.theme.panel_bg());
    frame.render_widget(footer_hint, inner_chunks[2]);

    frame.set_cursor_position((
        inner_chunks[1].x + app.ai_key_input.chars().count() as u16 + 1,
        inner_chunks[1].y + 1,
    ));
}

fn render_theme_selector(frame: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(40, 65, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .bg(app.theme.panel_bg())
        .title(Line::from(vec![
            Span::styled(" Themes ", Style::default().fg(app.theme.accent()).bold()),
        ]));
    frame.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area).inner(Margin { horizontal: 1, vertical: 0 });

    let total = app.theme_names.len();

    let rows: Vec<Row> = app.theme_names.iter()
        .enumerate()
        .map(|(abs_idx, name)| {
            let is_current = abs_idx == app.theme_index;
            let is_selected = abs_idx == app.theme_selector_row;
            let bg = if is_selected {
                app.theme.highlight_bg()
            } else {
                app.theme.panel_bg()
            };
            let prefix = if is_current { "\u{2726} " } else { "  " };
            let style = if is_selected {
                Style::default().fg(app.theme.accent()).bold().bg(bg)
            } else {
                Style::default().fg(app.theme.fg()).bg(bg)
            };
            Row::new(vec![
                Cell::from(format!("{}{}", prefix, name)).style(style),
            ]).bg(bg)
        })
        .collect();

    let content_height = total as u16;
    let table = Table::new(rows, &[Constraint::Percentage(100)])
        .column_spacing(0);

    let mut sv = ScrollView::new(Size::new(inner.width, content_height.max(1)))
        .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
    sv.render_widget(table, Rect::new(0, 0, inner.width, content_height));
    frame.render_stateful_widget(sv, inner, &mut app.theme_sv);
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

fn get_value_style(key: &str, value: &str, theme: &AppTheme) -> Style {
    if key.contains("Win Rate") || key.contains("Headshots") {
        if let Ok(num) = value.trim_end_matches('%').parse::<f64>() {
            if num >= 60.0 { return Style::default().fg(theme.success()).bold(); }
            else if num >= 40.0 { return Style::default().fg(theme.warn()); }
            else { return Style::default().fg(theme.error()); }
        }
    }
    if key.contains("K/D") || key.contains("ADR") || key.contains("Rating") {
        if let Ok(num) = value.parse::<f64>() {
            if num >= 1.2 { return Style::default().fg(theme.success()).bold(); }
            else if num >= 0.9 { return Style::default().fg(theme.warn()); }
            else { return Style::default().fg(theme.error()); }
        }
    }
    Style::default().fg(theme.fg())
}

fn match_key_order(key: &str) -> u32 {
    let priority: &[&str] = &[
        "Map", "Score", "Result", "Kills", "Deaths", "Assists",
        "K/D Ratio", "K/R Ratio", "Headshots", "Headshots %",
        "MVPs", "Triple Kills", "Quadro Kills", "Penta Kills",
        "ADR", "Damage", "Utility Damage", "Flashbangs",
        "Entry Count", "Entry Wins", "First Kills",
        "Clutches", "Team", "Region", "Match ID",
    ];
    priority.iter()
        .position(|&k| k == key)
        .map(|i| i as u32)
        .unwrap_or_else(|| {
            priority.len() as u32 + key.bytes().fold(0u32, |a, b| a.wrapping_add(b as u32))
        })
}

fn is_timestamp_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.contains("time") || lower.contains("date") || lower.contains("_at")
        || lower.contains("created") || lower.contains("updated")
        || lower.contains("started") || lower.contains("finished")
        || lower.contains("duration")
}

fn fmt_timestamp(val: &str) -> Option<String> {
    // ISO 8601: "2024-07-15T14:30:00Z" or "2024-07-15 14:30:00"
    if val.len() >= 16 && val.as_bytes().get(4) == Some(&b'-') && val.as_bytes().get(7) == Some(&b'-') {
        let day = &val[8..10];
        let month = &val[5..7];
        let year = &val[0..4];
        let time = if val.len() >= 16 {
            let sep = if val.as_bytes().get(10) == Some(&b'T') { 11 } else if val.as_bytes().get(10) == Some(&b' ') { 11 } else { 0 };
            if sep > 0 { format!(" {}", &val[sep..sep+5]) } else { String::new() }
        } else {
            String::new()
        };
        return Some(format!("{}.{}.{}{}", day, month, year, time));
    }
    None
}

fn clean_error_msg(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("401") || lower.contains("unauthorized") {
        "Invalid API key. Check your config.toml".into()
    } else if lower.contains("403") || lower.contains("forbidden") {
        "Access denied. Check API key permissions".into()
    } else if lower.contains("404") || lower.contains("not found") {
        "Player not found".into()
    } else if lower.contains("429") || lower.contains("rate limit") {
        "Too many requests. Wait a moment".into()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "Request timed out. Check your connection".into()
    } else if lower.contains("connection") || lower.contains("dns") || lower.contains("resolve") {
        "Network error. Check your internet".into()
    } else {
        format!("Something went wrong")
    }
}
