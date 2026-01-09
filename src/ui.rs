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
use crate::markdown::{estimate_rendered_lines, render_markdown_to_lines};
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

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .split(frame.size());

            let title = Span::styled(
                format!("{}  (q to quit, j/k or arrows to scroll)", path),
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

            let lines = render_markdown_to_lines(markdown, content_chunks[0].width, &theme);
            let lines = if enable_beeline {
                apply_beeline(&lines, &theme)
            } else {
                lines
            };
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

            let help = Line::raw("Up/Down, j/k, Space/Tab for page down, Shift+Tab for page up • q to quit")
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
                let status = Line::from(vec![
                    Span::styled(
                        format!("{}/{}", scroll.saturating_add(1), total_lines),
                        Style::new().fg(theme.footer).dim(),
                    ),
                    Span::raw(" "),
                    Span::styled(format!("{}%", percent), Style::new().fg(theme.footer).dim()),
                ]);
                frame.render_widget(Paragraph::new(status), footer_chunks[1]);
            }
        })?;

        match event::read()? {
            Event::Key(key) => {
                let page = viewport_height.saturating_sub(1).max(1);
                let max_scroll = rendered_lines.saturating_sub(viewport_height);
                match key.code {
                    KeyCode::Char('q') => break,
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
