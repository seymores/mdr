use std::io;
use std::process::Command;

use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, event::DisableMouseCapture, event::EnableMouseCapture};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Terminal;

use crate::beeline::apply_beeline;
use crate::markdown::{
    estimate_rendered_lines, render_markdown_with_links, render_plain_lines, LinkTarget,
};
use crate::theme::Theme;

pub fn run_tui(path: &str, markdown: &str, enable_beeline: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = Theme::pastel();

    let mut state = AppState::new(enable_beeline);

    loop {
        terminal.draw(|frame| state.render(frame, path, markdown, &theme))?;
        let event = event::read()?;
        if state.handle_event(event, &mut terminal)? {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

struct AppState {
    scroll: u16,
    viewport_height: u16,
    rendered_lines: u16,
    beeline_enabled: bool,
    plain_mode: bool,
    show_help: bool,
    search_mode: bool,
    search_query: String,
    search_matches: Vec<SearchMatch>,
    search_index: usize,
    current_links: Vec<LinkTarget>,
    current_line_offsets: Vec<u16>,
    content_area: Rect,
    hover_link: Option<String>,
}

impl AppState {
    fn new(enable_beeline: bool) -> Self {
        Self {
            scroll: 0,
            viewport_height: 0,
            rendered_lines: 0,
            beeline_enabled: enable_beeline,
            plain_mode: false,
            show_help: false,
            search_mode: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_index: 0,
            current_links: Vec::new(),
            current_line_offsets: Vec::new(),
            content_area: Rect::default(),
            hover_link: None,
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, path: &str, markdown: &str, theme: &Theme) {
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
        self.content_area = content_chunks[0];

        if self.show_help {
            let help_lines = help_lines();
            self.render_lines(frame, &help_lines, content_chunks[0]);
        } else {
            let mut lines = if self.plain_mode {
                self.current_links.clear();
                render_plain_lines(markdown)
            } else {
                let (lines, links) =
                    render_markdown_with_links(markdown, content_chunks[0].width, theme);
                self.current_links = links;
                lines
            };
            if self.beeline_enabled && !self.plain_mode {
                lines = apply_beeline(&lines, theme);
            }

            let lines_text: Vec<String> = lines
                .iter()
                .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect())
                .collect();
            self.current_line_offsets = line_offsets(&lines, content_chunks[0].width);

            if self.search_query.is_empty() {
                reset_search(&mut self.search_query, &mut self.search_matches, &mut self.search_index);
            } else {
                self.search_matches = build_search_matches(
                    &lines_text,
                    &self.search_query,
                    &self.current_line_offsets,
                    content_chunks[0].width,
                );
                if self.search_index >= self.search_matches.len() {
                    self.search_index = 0;
                }
                let active = self.search_matches.get(self.search_index);
                lines = apply_search_highlight(&lines, &self.search_query, active, theme);
            }

            self.render_lines(frame, &lines, content_chunks[0]);
        }

        if self.rendered_lines > self.viewport_height {
            let scroll_len = self
                .rendered_lines
                .saturating_sub(self.viewport_height)
                .saturating_add(1)
                .max(1);
            let mut scrollbar_state = ScrollbarState::new(scroll_len as usize)
                .position(self.scroll as usize)
                .viewport_content_length(self.viewport_height as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::new().fg(theme.scrollbar_thumb))
                .track_style(Style::new().fg(theme.scrollbar_track));
            frame.render_stateful_widget(scrollbar, content_chunks[1], &mut scrollbar_state);
        }

        let footer_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(24)])
            .split(chunks[1]);

        let help = if self.search_mode {
            Line::raw(format!("/{}", self.search_query))
        } else if let Some(url) = &self.hover_link {
            Line::raw(format!("link: {}", url))
        } else {
            Line::raw("Press h for commands • / search • q quit")
        }
        .style(Style::new().fg(theme.footer).dim());
        frame.render_widget(Paragraph::new(help), footer_chunks[0]);

        let total_lines = self.rendered_lines.max(1);
        if self.rendered_lines > self.viewport_height {
            let max_scroll = total_lines.saturating_sub(self.viewport_height);
            let percent = if max_scroll == 0 {
                100
            } else {
                (self.scroll.saturating_mul(100) / max_scroll).min(100)
            };
            let mut status_spans = vec![Span::styled(
                format!("{}/{}", self.scroll.saturating_add(1), total_lines),
                Style::new().fg(theme.footer).dim(),
            )];
            if !self.search_query.is_empty() {
                status_spans.push(Span::raw(" "));
                if self.search_matches.is_empty() {
                    status_spans.push(Span::styled(
                        "no matches",
                        Style::new().fg(theme.footer).dim(),
                    ));
                } else {
                    status_spans.push(Span::styled(
                        format!("{}/{}", self.search_index + 1, self.search_matches.len()),
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
    }

    fn render_lines(&mut self, frame: &mut ratatui::Frame, lines: &[Line<'static>], area: Rect) {
        let content = Text::from(lines.to_vec());
        let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
        self.viewport_height = area.height;
        self.rendered_lines = estimate_rendered_lines(lines, area.width);
        let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        let paragraph = paragraph.scroll((self.scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn handle_event(
        &mut self,
        event: Event,
        _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<bool> {
        match event {
            Event::Key(key) => {
                let page = self.viewport_height.saturating_sub(1).max(1);
                let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
                match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Char('/') if !self.show_help => {
                        self.search_mode = true;
                        reset_search(
                            &mut self.search_query,
                            &mut self.search_matches,
                            &mut self.search_index,
                        );
                    }
                    KeyCode::Enter if self.search_mode => {
                        self.search_mode = false;
                        if let Some(pos) = self.search_matches.first().map(|m| m.scroll_pos) {
                            self.scroll = pos.min(max_scroll);
                        }
                    }
                    KeyCode::Esc if self.search_mode => {
                        self.search_mode = false;
                        reset_search(
                            &mut self.search_query,
                            &mut self.search_matches,
                            &mut self.search_index,
                        );
                    }
                    KeyCode::Esc if !self.search_mode => {
                        if self.show_help {
                            self.show_help = false;
                            self.scroll = 0;
                        }
                        reset_search(
                            &mut self.search_query,
                            &mut self.search_matches,
                            &mut self.search_index,
                        );
                    }
                    KeyCode::Backspace if self.search_mode => {
                        self.search_query.pop();
                        self.search_matches.clear();
                        self.search_index = 0;
                    }
                    KeyCode::Char(c) if self.search_mode => {
                        self.search_query.push(c);
                        self.search_matches.clear();
                        self.search_index = 0;
                    }
                    KeyCode::Char('n') if !self.search_mode && !self.show_help => {
                        if !self.search_matches.is_empty() {
                            self.search_index = (self.search_index + 1) % self.search_matches.len();
                            self.scroll = self.search_matches[self.search_index]
                                .scroll_pos
                                .min(max_scroll);
                        }
                    }
                    KeyCode::Char('N') if !self.search_mode && !self.show_help => {
                        if !self.search_matches.is_empty() {
                            if self.search_index == 0 {
                                self.search_index = self.search_matches.len() - 1;
                            } else {
                                self.search_index -= 1;
                            }
                            self.scroll = self.search_matches[self.search_index]
                                .scroll_pos
                                .min(max_scroll);
                        }
                    }
                    KeyCode::Char('h') => {
                        self.show_help = !self.show_help;
                        self.scroll = 0;
                        if self.show_help {
                            self.search_mode = false;
                        }
                    }
                    KeyCode::Char('b') => {
                        self.beeline_enabled = !self.beeline_enabled;
                    }
                    KeyCode::Char('m') => {
                        self.plain_mode = !self.plain_mode;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll = self.scroll.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll = self.scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.scroll = self.scroll.saturating_add(page);
                    }
                    KeyCode::PageUp => {
                        self.scroll = self.scroll.saturating_sub(page);
                    }
                    KeyCode::Char(' ') | KeyCode::Tab => {
                        self.scroll = self.scroll.saturating_add(page);
                    }
                    KeyCode::BackTab => {
                        self.scroll = self.scroll.saturating_sub(page);
                    }
                    KeyCode::Home => {
                        self.scroll = 0;
                    }
                    KeyCode::End => {
                        self.scroll = max_scroll;
                    }
                    KeyCode::Enter if !self.search_mode && !self.show_help => {
                        if let Some(url) =
                            link_at_scroll(&self.current_links, &self.current_line_offsets, self.scroll)
                        {
                            let _ = open_url(&url);
                        }
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
                match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        self.scroll = self.scroll.saturating_add(3).min(max_scroll);
                        self.hover_link = update_hover(
                            &self.current_links,
                            &self.current_line_offsets,
                            self.content_area,
                            self.scroll,
                            mouse.column,
                            mouse.row,
                        );
                    }
                    MouseEventKind::ScrollUp => {
                        self.scroll = self.scroll.saturating_sub(3);
                        self.hover_link = update_hover(
                            &self.current_links,
                            &self.current_line_offsets,
                            self.content_area,
                            self.scroll,
                            mouse.column,
                            mouse.row,
                        );
                    }
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        self.hover_link = update_hover(
                            &self.current_links,
                            &self.current_line_offsets,
                            self.content_area,
                            self.scroll,
                            mouse.column,
                            mouse.row,
                        );
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        if !self.show_help
                            && !self.search_mode
                            && mouse.column >= self.content_area.x
                            && mouse.column < self.content_area.x + self.content_area.width
                            && mouse.row >= self.content_area.y
                            && mouse.row < self.content_area.y + self.content_area.height
                        {
                            let local_y = mouse.row.saturating_sub(self.content_area.y);
                            let rendered_line = self.scroll.saturating_add(local_y);
                            let local_x = mouse.column.saturating_sub(self.content_area.x);
                            self.hover_link = link_at_position(
                                &self.current_links,
                                &self.current_line_offsets,
                                self.content_area.width,
                                rendered_line,
                                local_x,
                            );
                            if let Some(url) = self.hover_link.clone() {
                                let _ = open_url(&url);
                            }
                        }
                    }
                    MouseEventKind::Up(_) => {
                        self.hover_link = update_hover(
                            &self.current_links,
                            &self.current_line_offsets,
                            self.content_area,
                            self.scroll,
                            mouse.column,
                            mouse.row,
                        );
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(false)
    }
}

#[derive(Clone, Copy)]
struct SearchMatch {
    line_idx: usize,
    start: usize,
    end: usize,
    start_char: usize,
    scroll_pos: u16,
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

fn find_matches(lines: &[String], query: &str) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }
    lines
        .iter()
        .enumerate()
        .flat_map(|(idx, line)| {
            match_ranges(line, query)
                .into_iter()
                .map(move |(start, end)| SearchMatch {
                    line_idx: idx,
                    start,
                    end,
                    start_char: line[..start].chars().count(),
                    scroll_pos: 0,
                })
        })
        .collect()
}

fn apply_search_highlight(
    lines: &[Line<'static>],
    query: &str,
    active: Option<&SearchMatch>,
    theme: &Theme,
) -> Vec<Line<'static>> {
    if query.is_empty() {
        return lines.to_vec();
    }
    lines
        .iter()
        .enumerate()
        .map(|(idx, line)| apply_search_highlight_line(line, query, active, idx, theme))
        .collect()
}

fn apply_search_highlight_line(
    line: &Line<'static>,
    query: &str,
    active: Option<&SearchMatch>,
    line_index: usize,
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
    let mut active_highlights = vec![false; char_offsets.len()];
    for (start, end) in ranges {
        let is_active = active
            .map(|m| m.line_idx == line_index && m.start == start && m.end == end)
            .unwrap_or(false);
        for (idx, &byte_offset) in char_offsets.iter().enumerate() {
            if byte_offset >= start && byte_offset < end {
                highlights[idx] = true;
                if is_active {
                    active_highlights[idx] = true;
                }
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
                let highlight = if active_highlights
                    .get(char_index)
                    .copied()
                    .unwrap_or(false)
                {
                    Style::new()
                        .bg(theme.search_bg_active)
                        .fg(theme.search_fg_active)
                } else {
                    Style::new().bg(theme.search_bg).fg(theme.search_fg)
                };
                style = style.patch(highlight);
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

fn build_search_matches(
    lines: &[String],
    query: &str,
    offsets: &[u16],
    width: u16,
) -> Vec<SearchMatch> {
    let width = width.max(1) as usize;
    let mut matches = find_matches(lines, query);
    for m in &mut matches {
        if let Some(&base) = offsets.get(m.line_idx) {
            let row = (m.start_char / width) as u16;
            m.scroll_pos = base.saturating_add(row);
        }
    }
    matches
}

fn reset_search(query: &mut String, matches: &mut Vec<SearchMatch>, index: &mut usize) {
    query.clear();
    matches.clear();
    *index = 0;
}

fn line_from_rendered(offsets: &[u16], rendered_line: u16) -> Option<(usize, u16)> {
    if offsets.is_empty() {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = offsets.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if offsets[mid] <= rendered_line {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let idx = lo.saturating_sub(1);
    offsets
        .get(idx)
        .map(|offset| (idx, rendered_line.saturating_sub(*offset)))
}

fn link_at_scroll(links: &[LinkTarget], offsets: &[u16], scroll: u16) -> Option<String> {
    if links.is_empty() {
        return None;
    }
    let mut best: Option<&LinkTarget> = None;
    for link in links {
        if let Some(&offset) = offsets.get(link.line_idx) {
            if offset >= scroll {
                best = Some(link);
                break;
            }
        }
    }
    best.or_else(|| links.first()).map(|l| l.url.clone())
}

fn link_at_position(
    links: &[LinkTarget],
    offsets: &[u16],
    width: u16,
    rendered_line: u16,
    column: u16,
) -> Option<String> {
    let (line_idx, row) = line_from_rendered(offsets, rendered_line)?;
    let width = width.max(1) as usize;
    let col = column as usize;
    let char_index = row as usize * width + col;
    links
        .iter()
        .find(|link| {
            link.line_idx == line_idx
                && char_index >= link.start_char
                && char_index < link.end_char
        })
        .map(|link| link.url.clone())
}

fn update_hover(
    links: &[LinkTarget],
    offsets: &[u16],
    area: Rect,
    scroll: u16,
    column: u16,
    row: u16,
) -> Option<String> {
    if links.is_empty() {
        return None;
    }
    if column < area.x
        || column >= area.x + area.width
        || row < area.y
        || row >= area.y + area.height
    {
        return None;
    }
    let local_y = row.saturating_sub(area.y);
    let rendered_line = scroll.saturating_add(local_y);
    let local_x = column.saturating_sub(area.x);
    link_at_position(links, offsets, area.width, rendered_line, local_x)
}

fn open_url(url: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", url]);
        c
    };
    cmd.status().map(|_| ())
}
