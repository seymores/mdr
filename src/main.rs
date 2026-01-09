use std::env;
use std::fs;
use std::io;
use std::process;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use pulldown_cmark::{Event as MdEvent, Options, Parser, Tag};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Usage: mdr <path-to-markdown>");
            process::exit(2);
        }
    };

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read {}: {}", path, err);
            process::exit(1);
        }
    };

    if let Err(err) = run_tui(&path, &content) {
        eprintln!("TUI error: {}", err);
        process::exit(1);
    }
}

fn run_tui(path: &str, markdown: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let text = render_markdown_to_text(markdown);
    let lines: Vec<Line> = text.lines().map(Line::raw).collect();
    let mut scroll: u16 = 0;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .split(frame.size());

            let title = format!("{}  (q to quit, j/k or arrows to scroll)", path);
            let content = Text::from(lines.clone());
            let paragraph = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title(title))
                .scroll((scroll, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, chunks[0]);

            let help = Line::raw("Up/Down or j/k to scroll • q to quit").style(Style::new().dim());
            let footer = Paragraph::new(help);
            frame.render_widget(footer, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down | KeyCode::Char('j') => {
                        scroll = scroll.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        scroll = scroll.saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_markdown_to_text(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut out = String::new();
    let mut list_depth = 0usize;

    for event in parser {
        match event {
            MdEvent::Start(Tag::Heading(level, ..)) => {
                out.push_str(&"#".repeat(level as usize));
                out.push(' ');
            }
            MdEvent::End(Tag::Heading(..)) => out.push_str("\n\n"),
            MdEvent::Start(Tag::Paragraph) => {}
            MdEvent::End(Tag::Paragraph) => out.push_str("\n\n"),
            MdEvent::Start(Tag::List(_)) => list_depth += 1,
            MdEvent::End(Tag::List(_)) => {
                if list_depth > 0 {
                    list_depth -= 1;
                }
                out.push('\n');
            }
            MdEvent::Start(Tag::Item) => {
                if list_depth > 0 {
                    out.push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                }
                out.push_str("- ");
            }
            MdEvent::End(Tag::Item) => out.push('\n'),
            MdEvent::Text(text) => out.push_str(&text),
            MdEvent::Code(code) => {
                out.push('`');
                out.push_str(&code);
                out.push('`');
            }
            MdEvent::SoftBreak => out.push('\n'),
            MdEvent::HardBreak => out.push_str("\n\n"),
            MdEvent::Rule => out.push_str("\n---\n"),
            MdEvent::Start(Tag::BlockQuote) => out.push_str("> "),
            MdEvent::End(Tag::BlockQuote) => out.push('\n'),
            MdEvent::Start(Tag::Emphasis) => out.push('*'),
            MdEvent::End(Tag::Emphasis) => out.push('*'),
            MdEvent::Start(Tag::Strong) => out.push_str("**"),
            MdEvent::End(Tag::Strong) => out.push_str("**"),
            _ => {}
        }
    }

    out.trim_end().to_string()
}
