use std::io;

use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, event::DisableMouseCapture, event::EnableMouseCapture};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Terminal;

use crate::beeline::apply_beeline;
use crate::markdown::{estimate_rendered_lines, render_markdown_to_lines, render_plain_lines};
use crate::theme::Theme;

pub fn run_tui(path: &str, markdown: &str, enable_beeline: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = Theme::pastel();

    let mut scroll: u16 = 0;
    let mut viewport_height: u16 = 0;
    let mut rendered_lines: u16 = 0;
    let mut beeline_enabled = enable_beeline;
    let mut plain_mode = false;
    let mut show_help = false;
    let mut search_mode = false;
    let mut search_query = String::new();
    let mut search_matches: Vec<u16> = Vec::new();
    let mut search_index: usize = 0;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .split(frame.size());

            let title = Span::styled(
                format!("{}", path),
                Style::new().fg(theme.title).add_modifier(Modifier::BOLD),
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(theme.border))
                .title(title);
            frame.render_widget(&block, chunks[0]);

            let inner = block.inner(chunks[0]);
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            if show_help {
                let help_lines = help_lines();
                let content = Text::from(help_lines.clone());
                let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
                viewport_height = content_chunks[0].height;
                rendered_lines = estimate_rendered_lines(&help_lines, content_chunks[0].width);
                let max_scroll = rendered_lines.saturating_sub(viewport_height);
                if scroll > max_scroll {
                    scroll = max_scroll;
                }
                let paragraph = paragraph.scroll((scroll, 0));
                frame.render_widget(paragraph, content_chunks[0]);
            } else {
                let mut lines = if plain_mode {
                    render_plain_lines(markdown)
                } else {
                    render_markdown_to_lines(markdown, content_chunks[0].width, &theme)
                };
                if beeline_enabled && !plain_mode {
                    lines = apply_beeline(&lines, &theme);
                }
                let lines_text: Vec<String> = lines
                    .iter()
                    .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect())
                    .collect();
                if search_query.is_empty() {
                    search_matches.clear();
                    search_index = 0;
                } else {
                    let match_lines = find_matches(&lines_text, &search_query);
                    let line_offsets = line_offsets(&lines, content_chunks[0].width);
                    search_matches = match_lines
                        .iter()
                        .filter_map(|line_idx| line_offsets.get(*line_idx).copied())
                        .collect();
                    if search_index >= search_matches.len() {
                        search_index = 0;
                    }
                }
                if !search_query.is_empty() {
                    lines = apply_search_highlight(&lines, &search_query, &theme);
                }
                let content = Text::from(lines.clone());
                let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
                viewport_height = content_chunks[0].height;
                rendered_lines = estimate_rendered_lines(&lines, content_chunks[0].width);
                let max_scroll = rendered_lines.saturating_sub(viewport_height);
                if scroll > max_scroll {
                    scroll = max_scroll;
                }
                let paragraph = paragraph.scroll((scroll, 0));
                frame.render_widget(paragraph, content_chunks[0]);
            }

            if rendered_lines > viewport_height {
                let scroll_len = rendered_lines
                    .saturating_sub(viewport_height)
                    .saturating_add(1)
                    .max(1);
                let mut scrollbar_state = ScrollbarState::new(scroll_len as usize)
                    .position(scroll as usize)
                    .viewport_content_length(viewport_height as usize);
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::new().fg(theme.scrollbar_thumb))
                    .track_style(Style::new().fg(theme.scrollbar_track));
                frame.render_stateful_widget(scrollbar, content_chunks[1], &mut scrollbar_state);
            }

            let footer_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(24)])
                .split(chunks[1]);

            let help = if search_mode {
                Line::raw(format!("/{}", search_query))
            } else {
                Line::raw("Press h for commands • / search • q quit")
            }
            .style(Style::new().fg(theme.footer).dim());
            frame.render_widget(Paragraph::new(help), footer_chunks[0]);

            let total_lines = rendered_lines.max(1);
            if rendered_lines > viewport_height {
                let max_scroll = total_lines.saturating_sub(viewport_height);
                let percent = if max_scroll == 0 {
                    100
                } else {
                    (scroll.saturating_mul(100) / max_scroll).min(100)
                };
                let mut status_spans = vec![Span::styled(
                    format!("{}/{}", scroll.saturating_add(1), total_lines),
                    Style::new().fg(theme.footer).dim(),
                )];
                if !search_query.is_empty() {
                    status_spans.push(Span::raw(" "));
                    if search_matches.is_empty() {
                        status_spans.push(Span::styled(
                            "no matches",
                            Style::new().fg(theme.footer).dim(),
                        ));
                    } else {
                        status_spans.push(Span::styled(
                            format!("{}/{}", search_index + 1, search_matches.len()),
                            Style::new().fg(theme.footer).dim(),
                        ));
                    }
                }
                status_spans.push(Span::raw(" "));
                status_spans.push(Span::styled(
                    format!("{}%", percent),
                    Style::new().fg(theme.footer).dim(),
                ));
                let status = Line::from(status_spans);
                frame.render_widget(Paragraph::new(status).right_aligned(), footer_chunks[1]);
            }
        })?;

        match event::read()? {
            Event::Key(key) => {
                let page = viewport_height.saturating_sub(1).max(1);
                let max_scroll = rendered_lines.saturating_sub(viewport_height);
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('/') if !show_help => {
                        search_mode = true;
                        search_query.clear();
                        search_matches.clear();
                        search_index = 0;
                    }
                    KeyCode::Enter if search_mode => {
                        search_mode = false;
                        if let Some(&line) = search_matches.first() {
                            scroll = line.min(max_scroll);
                        }
                    }
                    KeyCode::Esc if search_mode => {
                        search_mode = false;
                        search_query.clear();
                        search_matches.clear();
                        search_index = 0;
                    }
                    KeyCode::Esc if !search_mode => {
                        if show_help {
                            show_help = false;
                            scroll = 0;
                        }
                        search_query.clear();
                        search_matches.clear();
                        search_index = 0;
                    }
                    KeyCode::Backspace if search_mode => {
                        search_query.pop();
                        search_matches.clear();
                        search_index = 0;
                    }
                    KeyCode::Char(c) if search_mode => {
                        search_query.push(c);
                        search_matches.clear();
                        search_index = 0;
                    }
                    KeyCode::Char('n') if !search_mode && !show_help => {
                        if !search_matches.is_empty() {
                            search_index = (search_index + 1) % search_matches.len();
                            scroll = search_matches[search_index].min(max_scroll);
                        }
                    }
                    KeyCode::Char('N') if !search_mode && !show_help => {
                        if !search_matches.is_empty() {
                            if search_index == 0 {
                                search_index = search_matches.len() - 1;
                            } else {
                                search_index -= 1;
                            }
                            scroll = search_matches[search_index].min(max_scroll);
                        }
                    }
                    KeyCode::Char('h') => {
                        show_help = !show_help;
                        scroll = 0;
                    }
                    KeyCode::Char('b') => {
                        beeline_enabled = !beeline_enabled;
                    }
                    KeyCode::Char('m') => {
                        plain_mode = !plain_mode;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        scroll = scroll.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        scroll = scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        scroll = scroll.saturating_add(page);
                    }
                    KeyCode::PageUp => {
                        scroll = scroll.saturating_sub(page);
                    }
                    KeyCode::Char(' ') => {
                        scroll = scroll.saturating_add(page);
                    }
                    KeyCode::Tab => {
                        scroll = scroll.saturating_add(page);
                    }
                    KeyCode::BackTab => {
                        scroll = scroll.saturating_sub(page);
                    }
                    KeyCode::Home => {
                        scroll = 0;
                    }
                    KeyCode::End => {
                        scroll = max_scroll;
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                let max_scroll = rendered_lines.saturating_sub(viewport_height);
                match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        scroll = scroll.saturating_add(3).min(max_scroll);
                    }
                    MouseEventKind::ScrollUp => {
                        scroll = scroll.saturating_sub(3);
                    }
                    _ => {}
                }
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "mdr - help",
            Style::new().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw("Navigation:"),
        Line::raw("  Up/Down              Scroll line by line"),
        Line::raw("  Space                Page down"),
        Line::raw("  Mouse wheel          Scroll"),
        Line::raw(""),
        Line::raw("Search:"),
        Line::raw("  /                    Start search"),
        Line::raw("  Enter                Jump to first match"),
        Line::raw("  Esc                  Cancel search"),
        Line::raw("  n / N                Next/previous match"),
        Line::raw(""),
        Line::raw("Modes:"),
        Line::raw("  b                    Toggle BeeLine"),
        Line::raw("  m                    Toggle plain mode"),
        Line::raw(""),
        Line::raw("General:"),
        Line::raw("  h                    Toggle help"),
        Line::raw("  q                    Quit"),
    ]
}

fn find_matches(lines: &[String], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| if !match_ranges(line, query).is_empty() { Some(idx) } else { None })
        .collect()
}

fn apply_search_highlight(
    lines: &[Line<'static>],
    query: &str,
    theme: &Theme,
) -> Vec<Line<'static>> {
    if query.is_empty() {
        return lines.to_vec();
    }
    lines
        .iter()
        .map(|line| apply_search_highlight_line(line, query, theme))
        .collect()
}

fn apply_search_highlight_line(
    line: &Line<'static>,
    query: &str,
    theme: &Theme,
) -> Line<'static> {
    let line_text: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    let ranges = match_ranges(&line_text, query);
    if ranges.is_empty() {
        return line.clone();
    }

    let char_offsets: Vec<usize> = line_text.char_indices().map(|(i, _)| i).collect();
    let mut highlights = vec![false; char_offsets.len()];
    for (start, end) in ranges {
        for (idx, &byte_offset) in char_offsets.iter().enumerate() {
            if byte_offset >= start && byte_offset < end {
                highlights[idx] = true;
            }
        }
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_style: Option<Style> = None;
    let mut char_index = 0usize;

    for span in &line.spans {
        for ch in span.content.chars() {
            let mut style = span.style;
            if highlights.get(char_index).copied().unwrap_or(false) {
                style = style.patch(Style::new().bg(theme.search_bg).fg(theme.search_fg));
            }
            if current_style == Some(style) {
                current.push(ch);
            } else {
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), current_style.unwrap_or_default()));
                    current.clear();
                }
                current_style = Some(style);
                current.push(ch);
            }
            char_index += 1;
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, current_style.unwrap_or_default()));
    }

    Line {
        spans,
        style: line.style,
        alignment: line.alignment,
    }
}

fn match_ranges(line: &str, query: &str) -> Vec<(usize, usize)> {
    let hay = line.as_bytes();
    let needle = query.as_bytes();
    if needle.is_empty() || needle.len() > hay.len() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    for i in 0..=hay.len() - needle.len() {
        let mut matched = true;
        for j in 0..needle.len() {
            if hay[i + j].to_ascii_lowercase() != needle[j].to_ascii_lowercase() {
                matched = false;
                break;
            }
        }
        if matched {
            ranges.push((i, i + needle.len()));
        }
    }
    ranges
}

fn line_offsets(lines: &[Line<'static>], width: u16) -> Vec<u16> {
    let mut offsets: Vec<u16> = Vec::with_capacity(lines.len());
    let mut current: u16 = 0;
    let width = width.max(1) as usize;
    for line in lines {
        offsets.push(current);
        let line_width = line.width().max(1);
        let wrapped = (line_width + width - 1) / width;
        current = current.saturating_add(wrapped as u16);
    }
    offsets
}
