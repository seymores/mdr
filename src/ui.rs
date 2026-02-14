use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{
    event::DisableFocusChange, event::DisableMouseCapture, event::EnableFocusChange,
    event::EnableMouseCapture, execute,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use unicode_width::UnicodeWidthChar;

use crate::beeline::apply_beeline;
use crate::document_queue::{DocumentQueue, QueuedDocument};
use crate::markdown::{
    LinkTarget, estimate_rendered_lines, render_markdown_with_links, render_plain_lines,
};
use crate::picker::{PickerEntry, PickerEntryKind, list_entries};
use crate::theme::Theme;

pub fn run_tui(
    mut queue: DocumentQueue,
    picker_root: PathBuf,
    enable_beeline: bool,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, EnableMouseCapture, EnableFocusChange)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    execute!(
        terminal.backend_mut(),
        EnableMouseCapture,
        EnableFocusChange
    )?;
    let theme = Theme::pastel();

    let mut state = AppState::new(enable_beeline);

    loop {
        {
            let current = queue.current();
            let path = current.path.display().to_string();
            let queue_index = queue.current_index();
            let queue_len = queue.len();
            let queue_paths: Vec<String> = queue
                .documents()
                .iter()
                .map(|doc| doc.path.display().to_string())
                .collect();
            let context = RenderContext {
                path: &path,
                markdown: &current.content,
                queue_index,
                queue_len,
                queue_paths: &queue_paths,
            };
            terminal.draw(|frame| state.render(frame, &context, &theme))?;
        }

        if state.priming_mode {
            if event::poll(Duration::from_millis(80))? {
                let event = event::read()?;
                match state.handle_event(event, &mut terminal)? {
                    EventResult::Quit => break,
                    EventResult::NextDocument => {
                        queue.next();
                        state.on_document_changed();
                    }
                    EventResult::PreviousDocument => {
                        queue.prev();
                        state.on_document_changed();
                    }
                    EventResult::OpenPicker => {
                        state.open_picker(picker_root.clone());
                    }
                    EventResult::OpenGoDialog => {
                        state.open_go_dialog(queue.len(), queue.current_index());
                    }
                    EventResult::OpenPath(path) => {
                        let mut switched = queue.focus_existing(&path);
                        if !switched && let Ok(content) = fs::read_to_string(&path) {
                            queue.push_and_focus(QueuedDocument::new(path, content));
                            switched = true;
                        }
                        if switched {
                            state.on_document_changed();
                        }
                    }
                    EventResult::GoToIndex(index) => {
                        if queue.focus_index(index) {
                            state.on_document_changed();
                        }
                    }
                    EventResult::Continue => {}
                }
                execute!(
                    terminal.backend_mut(),
                    DisableMouseCapture,
                    EnableMouseCapture
                )?;
                state.priming_mode = false;
            } else {
                execute!(
                    terminal.backend_mut(),
                    DisableMouseCapture,
                    EnableMouseCapture
                )?;
                state.priming_mode = false;
            }
        } else {
            let event = event::read()?;
            match state.handle_event(event, &mut terminal)? {
                EventResult::Quit => break,
                EventResult::NextDocument => {
                    queue.next();
                    state.on_document_changed();
                }
                EventResult::PreviousDocument => {
                    queue.prev();
                    state.on_document_changed();
                }
                EventResult::OpenPicker => {
                    state.open_picker(picker_root.clone());
                }
                EventResult::OpenGoDialog => {
                    state.open_go_dialog(queue.len(), queue.current_index());
                }
                EventResult::OpenPath(path) => {
                    let mut switched = queue.focus_existing(&path);
                    if !switched && let Ok(content) = fs::read_to_string(&path) {
                        queue.push_and_focus(QueuedDocument::new(path, content));
                        switched = true;
                    }
                    if switched {
                        state.on_document_changed();
                    }
                }
                EventResult::GoToIndex(index) => {
                    if queue.focus_index(index) {
                        state.on_document_changed();
                    }
                }
                EventResult::Continue => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        DisableFocusChange,
        LeaveAlternateScreen
    )?;
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
    current_wraps: Vec<LineWrap>,
    current_lines_text: Vec<String>,
    scroll_before_help: Option<u16>,
    content_area: Rect,
    hover_link: Option<String>,
    last_mouse_pos: Option<(u16, u16)>,
    priming_mode: bool,
    picker_open: bool,
    picker_query: String,
    picker_dir: PathBuf,
    picker_entries: Vec<PickerEntry>,
    picker_selected: usize,
    go_dialog_open: bool,
    go_dialog_total: usize,
    go_dialog_selected: usize,
}

struct RenderContext<'a> {
    path: &'a str,
    markdown: &'a str,
    queue_index: usize,
    queue_len: usize,
    queue_paths: &'a [String],
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
            current_wraps: Vec::new(),
            current_lines_text: Vec::new(),
            scroll_before_help: None,
            content_area: Rect::default(),
            hover_link: None,
            last_mouse_pos: None,
            priming_mode: true,
            picker_open: false,
            picker_query: String::new(),
            picker_dir: PathBuf::new(),
            picker_entries: Vec::new(),
            picker_selected: 0,
            go_dialog_open: false,
            go_dialog_total: 0,
            go_dialog_selected: 0,
        }
    }

    fn close_help(&mut self) {
        if self.show_help {
            self.show_help = false;
        }
    }

    fn on_document_changed(&mut self) {
        self.scroll = 0;
        self.search_mode = false;
        self.clear_search_state();
        self.hover_link = None;
        self.show_help = false;
        self.scroll_before_help = None;
        self.current_links.clear();
        self.current_line_offsets.clear();
        self.current_wraps.clear();
        self.current_lines_text.clear();
        self.close_picker();
        self.close_go_dialog();
    }

    fn open_picker(&mut self, start_dir: PathBuf) {
        self.close_go_dialog();
        self.picker_open = true;
        self.picker_query.clear();
        self.picker_dir = fs::canonicalize(&start_dir).unwrap_or(start_dir);
        self.picker_selected = 0;
        self.search_mode = false;
        self.hover_link = None;
        self.refresh_picker_entries();
    }

    fn close_picker(&mut self) {
        self.picker_open = false;
        self.picker_query.clear();
        self.picker_entries.clear();
        self.picker_dir = PathBuf::new();
        self.picker_selected = 0;
    }

    fn open_go_dialog(&mut self, total: usize, current_index: usize) {
        self.close_picker();
        self.go_dialog_open = total > 0;
        self.go_dialog_total = total;
        self.go_dialog_selected = if total == 0 {
            0
        } else {
            current_index.min(total - 1)
        };
        self.search_mode = false;
        self.show_help = false;
        self.hover_link = None;
    }

    fn close_go_dialog(&mut self) {
        self.go_dialog_open = false;
        self.go_dialog_total = 0;
        self.go_dialog_selected = 0;
    }

    fn refresh_picker_entries(&mut self) {
        self.picker_entries =
            list_entries(self.picker_dir.clone(), &self.picker_query).unwrap_or_default();
        if self.picker_selected >= self.picker_entries.len() {
            self.picker_selected = self.picker_entries.len().saturating_sub(1);
        }
    }

    fn handle_key_input(&mut self, code: KeyCode, max_scroll: u16, page: u16) -> KeyAction {
        match code {
            KeyCode::Char('q') => KeyAction::Quit,
            KeyCode::Char('/') if !self.show_help => {
                self.search_mode = true;
                self.clear_search_state();
                KeyAction::None
            }
            KeyCode::Enter if self.search_mode => {
                self.search_mode = false;
                if let Some(pos) = self.search_matches.first().map(|m| m.scroll_pos) {
                    self.scroll = pos.min(max_scroll);
                }
                KeyAction::None
            }
            KeyCode::Esc if self.search_mode => {
                self.search_mode = false;
                self.clear_search_state();
                KeyAction::None
            }
            KeyCode::Esc if !self.search_mode => {
                self.close_help();
                self.clear_search_state();
                KeyAction::None
            }
            KeyCode::Backspace if self.search_mode => {
                self.search_query.pop();
                self.reset_search_matches();
                KeyAction::None
            }
            KeyCode::Char(c) if self.search_mode => {
                self.search_query.push(c);
                self.reset_search_matches();
                KeyAction::None
            }
            KeyCode::Char('n') if !self.search_mode && !self.show_help => {
                if !self.search_matches.is_empty() {
                    self.search_index = (self.search_index + 1) % self.search_matches.len();
                    self.scroll = self.search_matches[self.search_index]
                        .scroll_pos
                        .min(max_scroll);
                }
                KeyAction::None
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
                KeyAction::None
            }
            KeyCode::Char('h') => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.scroll_before_help = Some(self.scroll);
                    self.search_mode = false;
                    self.hover_link = None;
                }
                KeyAction::None
            }
            KeyCode::Char('b') => {
                self.beeline_enabled = !self.beeline_enabled;
                KeyAction::None
            }
            KeyCode::Char('m') => {
                self.plain_mode = !self.plain_mode;
                KeyAction::None
            }
            KeyCode::Char(']') if !self.search_mode && !self.show_help => KeyAction::NextDocument,
            KeyCode::Char('[') if !self.search_mode && !self.show_help => {
                KeyAction::PreviousDocument
            }
            KeyCode::Char('g') if !self.search_mode && !self.show_help => KeyAction::OpenGoDialog,
            KeyCode::Char('o') if !self.search_mode && !self.show_help => KeyAction::OpenPicker,
            KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                KeyAction::None
            }
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                KeyAction::None
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(page);
                KeyAction::None
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(page);
                KeyAction::None
            }
            KeyCode::Char(' ') | KeyCode::Tab => {
                self.scroll = self.scroll.saturating_add(page);
                KeyAction::None
            }
            KeyCode::BackTab => {
                self.scroll = self.scroll.saturating_sub(page);
                KeyAction::None
            }
            KeyCode::Home => {
                self.scroll = 0;
                KeyAction::None
            }
            KeyCode::End => {
                self.scroll = max_scroll;
                KeyAction::None
            }
            KeyCode::Enter if !self.search_mode && !self.show_help => KeyAction::OpenLink,
            _ => KeyAction::None,
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, context: &RenderContext<'_>, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .margin(1)
            .split(frame.size());

        let title_text = queue_label(context.queue_index, context.queue_len, context.path);
        let title = Span::styled(
            title_text,
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
            self.render_lines_with_scroll(frame, &help_lines, content_chunks[0], 0);
        } else {
            if let Some(prev) = self.scroll_before_help.take() {
                self.scroll = prev;
            }
            let mut lines = if self.plain_mode {
                self.current_links.clear();
                render_plain_lines(context.markdown)
            } else {
                let (lines, links) =
                    render_markdown_with_links(context.markdown, content_chunks[0].width, theme);
                self.current_links = links;
                lines
            };
            if self.beeline_enabled && !self.plain_mode {
                lines = apply_beeline(&lines, theme);
            }

            let lines_text: Vec<String> = lines
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect()
                })
                .collect();
            self.current_lines_text = lines_text;
            let (wraps, offsets) = build_wraps(&self.current_lines_text, content_chunks[0].width);
            self.current_wraps = wraps;
            self.current_line_offsets = offsets;

            if self.search_query.is_empty() {
                self.clear_search_state();
            } else {
                self.search_matches = build_search_matches(
                    &self.current_lines_text,
                    &self.search_query,
                    &self.current_wraps,
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

        if self.picker_open {
            self.render_picker_overlay(frame, chunks[0], theme);
        }
        if self.go_dialog_open {
            self.render_go_dialog_overlay(frame, chunks[0], context.queue_paths, theme);
        }
    }

    fn render_lines(&mut self, frame: &mut ratatui::Frame, lines: &[Line<'static>], area: Rect) {
        self.render_lines_with_scroll(frame, lines, area, self.scroll);
        let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    fn render_lines_with_scroll(
        &mut self,
        frame: &mut ratatui::Frame,
        lines: &[Line<'static>],
        area: Rect,
        scroll: u16,
    ) {
        let content = Text::from(lines.to_vec());
        let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
        self.viewport_height = area.height;
        self.rendered_lines = estimate_rendered_lines(lines, area.width);
        let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
        let scroll = scroll.min(max_scroll);
        let paragraph = paragraph.scroll((scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_picker_overlay(&self, frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
        let popup = centered_rect(80, 70, area);
        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(Span::styled(
                "Open Markdown (Filesystem)",
                Style::new().fg(theme.title).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme.border));
        frame.render_widget(block.clone(), popup);
        let inner = block.inner(popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let dir = Line::styled(
            format!("dir: {}", self.picker_dir.display()),
            Style::new().fg(theme.footer).dim(),
        );
        frame.render_widget(Paragraph::new(dir), chunks[0]);

        let query = Line::styled(
            format!("query: {}", self.picker_query),
            Style::new().fg(theme.footer),
        );
        frame.render_widget(Paragraph::new(query), chunks[1]);

        let mut lines = Vec::new();
        if self.picker_entries.is_empty() {
            lines.push(Line::styled(
                "No markdown files or directories found",
                Style::new().fg(theme.footer).dim(),
            ));
        } else {
            let visible = chunks[2].height.max(1) as usize;
            let start = self
                .picker_selected
                .saturating_sub(visible.saturating_sub(1));
            let end = (start + visible).min(self.picker_entries.len());
            for idx in start..end {
                let entry = &self.picker_entries[idx];
                let mut style = Style::new().fg(theme.footer);
                if idx == self.picker_selected {
                    style = style
                        .fg(theme.search_fg_active)
                        .bg(theme.search_bg_active)
                        .add_modifier(Modifier::BOLD);
                }
                lines.push(Line::styled(entry.label.clone(), style));
            }
        }
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[2]);

        let help = Line::styled(
            "Enter open/enter dir  Backspace up  Esc close",
            Style::new().fg(theme.footer).dim(),
        );
        frame.render_widget(Paragraph::new(help), chunks[3]);
    }

    fn render_go_dialog_overlay(
        &self,
        frame: &mut ratatui::Frame,
        area: Rect,
        queue_paths: &[String],
        theme: &Theme,
    ) {
        let popup = centered_rect(70, 60, area);
        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(Span::styled(
                "Go To Document",
                Style::new().fg(theme.title).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme.border));
        frame.render_widget(block.clone(), popup);
        let inner = block.inner(popup);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let mut lines = Vec::new();
        if queue_paths.is_empty() {
            lines.push(Line::styled(
                "Queue is empty",
                Style::new().fg(theme.footer).dim(),
            ));
        } else {
            let visible = chunks[0].height.max(1) as usize;
            let start = self
                .go_dialog_selected
                .saturating_sub(visible.saturating_sub(1));
            let end = (start + visible).min(queue_paths.len());
            for (idx, path) in queue_paths[start..end].iter().enumerate() {
                let absolute_idx = start + idx;
                let mut style = Style::new().fg(theme.footer);
                if absolute_idx == self.go_dialog_selected {
                    style = style
                        .fg(theme.search_fg_active)
                        .bg(theme.search_bg_active)
                        .add_modifier(Modifier::BOLD);
                }
                lines.push(Line::styled(
                    format!("[{}/{}] {}", absolute_idx + 1, queue_paths.len(), path),
                    style,
                ));
            }
        }
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);
        frame.render_widget(
            Paragraph::new(Line::styled(
                "Enter go  Esc close  Up/Down select",
                Style::new().fg(theme.footer).dim(),
            )),
            chunks[1],
        );
    }

    fn handle_event(
        &mut self,
        event: Event,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<EventResult> {
        match event {
            Event::Key(key) => {
                if self.picker_open {
                    return Ok(self.handle_picker_key_input(key.code));
                }
                if self.go_dialog_open {
                    return Ok(self.handle_go_dialog_key_input(key.code));
                }

                let page = self.viewport_height.saturating_sub(1).max(1);
                let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
                match self.handle_key_input(key.code, max_scroll, page) {
                    KeyAction::Quit => return Ok(EventResult::Quit),
                    KeyAction::OpenLink => {
                        if let Some(url) = link_at_scroll(
                            &self.current_links,
                            &self.current_wraps,
                            &self.current_line_offsets,
                            self.scroll,
                        ) {
                            let _ = open_url(&url);
                        }
                    }
                    KeyAction::NextDocument => return Ok(EventResult::NextDocument),
                    KeyAction::PreviousDocument => return Ok(EventResult::PreviousDocument),
                    KeyAction::OpenPicker => return Ok(EventResult::OpenPicker),
                    KeyAction::OpenGoDialog => return Ok(EventResult::OpenGoDialog),
                    KeyAction::None => {}
                }
            }
            Event::Mouse(mouse) => {
                self.last_mouse_pos = Some((mouse.column, mouse.row));
                let max_scroll = self.rendered_lines.saturating_sub(self.viewport_height);
                match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        self.scroll = self.scroll.saturating_add(3).min(max_scroll);
                        if !self.show_help {
                            self.hover_link = update_hover(self, mouse.column, mouse.row);
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        self.scroll = self.scroll.saturating_sub(3);
                        if !self.show_help {
                            self.hover_link = update_hover(self, mouse.column, mouse.row);
                        }
                    }
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        if !self.show_help {
                            self.hover_link = update_hover(self, mouse.column, mouse.row);
                        }
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
                                &self.current_wraps,
                                &self.current_line_offsets,
                                &self.current_lines_text,
                                rendered_line,
                                local_x,
                            );
                            if let Some(url) = self.hover_link.clone() {
                                let _ = open_url(&url);
                            }
                        }
                    }
                    MouseEventKind::Up(_) => {
                        if !self.show_help {
                            self.hover_link = update_hover(self, mouse.column, mouse.row);
                        }
                    }
                    _ => {}
                }
            }
            Event::FocusGained => {
                let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                if let Some((col, row)) = self.last_mouse_pos {
                    self.hover_link = update_hover(self, col, row);
                }
            }
            Event::FocusLost => {
                self.hover_link = None;
            }
            Event::Resize(_, _) => {
                let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                if let Some((col, row)) = self.last_mouse_pos {
                    self.hover_link = update_hover(self, col, row);
                }
            }
            _ => {}
        }
        Ok(EventResult::Continue)
    }

    fn handle_picker_key_input(&mut self, code: KeyCode) -> EventResult {
        match code {
            KeyCode::Esc => {
                self.close_picker();
                EventResult::Continue
            }
            KeyCode::Up | KeyCode::BackTab => {
                self.picker_selected = self.picker_selected.saturating_sub(1);
                EventResult::Continue
            }
            KeyCode::Down | KeyCode::Tab => {
                if !self.picker_entries.is_empty() {
                    self.picker_selected =
                        (self.picker_selected + 1).min(self.picker_entries.len() - 1);
                }
                EventResult::Continue
            }
            KeyCode::Home => {
                self.picker_selected = 0;
                EventResult::Continue
            }
            KeyCode::End => {
                self.picker_selected = self.picker_entries.len().saturating_sub(1);
                EventResult::Continue
            }
            KeyCode::Backspace => {
                if self.picker_query.is_empty() {
                    if let Some(parent) = self.picker_dir.parent() {
                        self.picker_dir = parent.to_path_buf();
                        self.picker_selected = 0;
                    }
                } else {
                    self.picker_query.pop();
                }
                self.refresh_picker_entries();
                EventResult::Continue
            }
            KeyCode::Char(c) => {
                self.picker_query.push(c);
                self.refresh_picker_entries();
                EventResult::Continue
            }
            KeyCode::Enter => {
                if let Some(entry) = self.picker_entries.get(self.picker_selected).cloned() {
                    match entry.kind {
                        PickerEntryKind::Parent | PickerEntryKind::Directory => {
                            self.picker_dir = entry.path;
                            self.picker_query.clear();
                            self.picker_selected = 0;
                            self.refresh_picker_entries();
                            EventResult::Continue
                        }
                        PickerEntryKind::MarkdownFile => {
                            self.close_picker();
                            EventResult::OpenPath(entry.path)
                        }
                    }
                } else {
                    EventResult::Continue
                }
            }
            _ => EventResult::Continue,
        }
    }

    fn handle_go_dialog_key_input(&mut self, code: KeyCode) -> EventResult {
        match code {
            KeyCode::Esc => {
                self.close_go_dialog();
                EventResult::Continue
            }
            KeyCode::Up | KeyCode::BackTab => {
                self.go_dialog_selected = self.go_dialog_selected.saturating_sub(1);
                EventResult::Continue
            }
            KeyCode::Down | KeyCode::Tab => {
                if self.go_dialog_total > 0 {
                    self.go_dialog_selected =
                        (self.go_dialog_selected + 1).min(self.go_dialog_total - 1);
                }
                EventResult::Continue
            }
            KeyCode::Home => {
                self.go_dialog_selected = 0;
                EventResult::Continue
            }
            KeyCode::End => {
                self.go_dialog_selected = self.go_dialog_total.saturating_sub(1);
                EventResult::Continue
            }
            KeyCode::Enter => {
                let selected = self.go_dialog_selected;
                self.close_go_dialog();
                EventResult::GoToIndex(selected)
            }
            _ => EventResult::Continue,
        }
    }

    fn clear_search_state(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.search_index = 0;
    }

    fn reset_search_matches(&mut self) {
        self.search_matches.clear();
        self.search_index = 0;
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

enum KeyAction {
    None,
    Quit,
    OpenLink,
    NextDocument,
    PreviousDocument,
    OpenPicker,
    OpenGoDialog,
}

enum EventResult {
    Continue,
    Quit,
    OpenPicker,
    OpenGoDialog,
    OpenPath(PathBuf),
    GoToIndex(usize),
    NextDocument,
    PreviousDocument,
}

#[derive(Clone)]
struct LineWrap {
    rows: Vec<RowRange>,
}

#[derive(Clone, Copy)]
struct RowRange {
    start: usize,
    end: usize,
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
        Line::raw("  ]                    Next document"),
        Line::raw("  [                    Previous document"),
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
        Line::raw("  g                    Go to document"),
        Line::raw("  o                    Open markdown filesystem browser"),
        Line::raw("  h                    Toggle help"),
        Line::raw("  q                    Quit"),
    ]
}

fn queue_label(current_index: usize, total: usize, path: &str) -> String {
    let total = total.max(1);
    let current = (current_index + 1).min(total);
    format!("[{}/{}] {}", current, total, path)
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
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
                let highlight = if active_highlights.get(char_index).copied().unwrap_or(false) {
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
                    spans.push(Span::styled(
                        current.clone(),
                        current_style.unwrap_or_default(),
                    ));
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
            if !hay[i + j].eq_ignore_ascii_case(&needle[j]) {
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

fn build_search_matches(
    lines: &[String],
    query: &str,
    wraps: &[LineWrap],
    offsets: &[u16],
    _width: u16,
) -> Vec<SearchMatch> {
    let mut matches = find_matches(lines, query);
    for m in &mut matches {
        if let (Some(&base), Some(wrap)) = (offsets.get(m.line_idx), wraps.get(m.line_idx)) {
            let row = row_for_char(wrap, m.start_char).unwrap_or(0) as u16;
            m.scroll_pos = base.saturating_add(row);
        }
    }
    matches
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

fn link_at_scroll(
    links: &[LinkTarget],
    wraps: &[LineWrap],
    offsets: &[u16],
    scroll: u16,
) -> Option<String> {
    if links.is_empty() {
        return None;
    }
    let mut best: Option<&LinkTarget> = None;
    for link in links {
        if let (Some(&offset), Some(wrap)) = (offsets.get(link.line_idx), wraps.get(link.line_idx))
        {
            let row = row_for_char(wrap, link.start_char).unwrap_or(0) as u16;
            let pos = offset.saturating_add(row);
            if pos >= scroll {
                best = Some(link);
                break;
            }
        }
    }
    best.or_else(|| links.first()).map(|l| l.url.clone())
}

fn link_at_position(
    links: &[LinkTarget],
    wraps: &[LineWrap],
    offsets: &[u16],
    lines_text: &[String],
    rendered_line: u16,
    column: u16,
) -> Option<String> {
    let (line_idx, row) = line_from_rendered(offsets, rendered_line)?;
    let wrap = wraps.get(line_idx)?;
    let row_range = wrap.rows.get(row as usize)?;
    let line_text = lines_text.get(line_idx)?;
    let char_index = char_index_at_col(line_text, row_range, column as usize)?;
    links
        .iter()
        .find(|link| {
            link.line_idx == line_idx && char_index >= link.start_char && char_index < link.end_char
        })
        .map(|link| link.url.clone())
}

fn update_hover(state: &AppState, column: u16, row: u16) -> Option<String> {
    if state.current_links.is_empty() {
        return None;
    }
    if column < state.content_area.x
        || column >= state.content_area.x + state.content_area.width
        || row < state.content_area.y
        || row >= state.content_area.y + state.content_area.height
    {
        return None;
    }
    let local_y = row.saturating_sub(state.content_area.y);
    let rendered_line = state.scroll.saturating_add(local_y);
    let local_x = column.saturating_sub(state.content_area.x);
    link_at_position(
        &state.current_links,
        &state.current_wraps,
        &state.current_line_offsets,
        &state.current_lines_text,
        rendered_line,
        local_x,
    )
}

fn char_index_at_col(line_text: &str, row_range: &RowRange, column: usize) -> Option<usize> {
    if row_range.start >= row_range.end {
        return None;
    }
    let mut col = 0usize;
    for (i, ch) in line_text.chars().enumerate() {
        if i < row_range.start {
            continue;
        }
        if i >= row_range.end {
            break;
        }
        let width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if column < col + width {
            return Some(i);
        }
        col = col.saturating_add(width);
    }
    None
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

fn build_wraps(lines: &[String], width: u16) -> (Vec<LineWrap>, Vec<u16>) {
    let width = width.max(1) as usize;
    let mut wraps: Vec<LineWrap> = Vec::with_capacity(lines.len());
    let mut offsets: Vec<u16> = Vec::with_capacity(lines.len());
    let mut current: u16 = 0;

    for line in lines {
        offsets.push(current);
        let wrap = wrap_line_ranges(line, width);
        let rows = wrap.rows.len().max(1) as u16;
        current = current.saturating_add(rows);
        wraps.push(wrap);
    }
    (wraps, offsets)
}

fn wrap_line_ranges(line: &str, width: usize) -> LineWrap {
    if line.is_empty() {
        return LineWrap {
            rows: vec![RowRange { start: 0, end: 0 }],
        };
    }
    let chars: Vec<char> = line.chars().collect();
    let widths: Vec<usize> = chars
        .iter()
        .map(|ch| UnicodeWidthChar::width(*ch).unwrap_or(0).max(1))
        .collect();
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut start = 0usize;
    let mut in_ws = chars[0].is_whitespace();
    for (i, ch) in chars.iter().enumerate() {
        let is_ws = ch.is_whitespace();
        if is_ws != in_ws {
            tokens.push((start, i));
            start = i;
            in_ws = is_ws;
        }
    }
    tokens.push((start, chars.len()));

    let mut rows: Vec<RowRange> = Vec::new();
    let mut row_start = 0usize;
    let mut row_width = 0usize;
    let mut row_end = 0usize;

    for (tok_start, tok_end) in tokens {
        let mut token_start = tok_start;
        let mut token_width: usize = widths[tok_start..tok_end].iter().sum();

        loop {
            let remaining = width.saturating_sub(row_width);
            if token_width <= remaining {
                if row_width == 0 {
                    row_start = token_start;
                }
                row_width = row_width.saturating_add(token_width);
                row_end = tok_end;
                break;
            }
            if row_width > 0 {
                rows.push(RowRange {
                    start: row_start,
                    end: row_end,
                });
                row_width = 0;
                continue;
            }
            let mut consumed = 0usize;
            let mut split_end = token_start;
            while split_end < tok_end {
                let w = widths[split_end];
                if consumed + w > width && consumed > 0 {
                    break;
                }
                consumed = consumed.saturating_add(w);
                split_end += 1;
                if consumed >= width {
                    break;
                }
            }
            let chunk_end = split_end.max(token_start + 1);
            rows.push(RowRange {
                start: token_start,
                end: chunk_end,
            });
            token_start = chunk_end;
            if token_start >= tok_end {
                break;
            }
            token_width = widths[token_start..tok_end].iter().sum();
        }
    }

    if row_width > 0 {
        rows.push(RowRange {
            start: row_start,
            end: row_end,
        });
    }

    LineWrap { rows }
}

fn row_for_char(wrap: &LineWrap, char_idx: usize) -> Option<usize> {
    wrap.rows
        .iter()
        .position(|row| char_idx >= row.start && char_idx < row.end)
}

#[cfg(test)]
mod tests;
