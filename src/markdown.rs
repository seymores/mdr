use pulldown_cmark::{Event as MdEvent, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

pub fn render_markdown_to_lines(
    markdown: &str,
    table_width: u16,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut list_depth = 0usize;
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut style_stack: Vec<Style> = Vec::new();
    let mut current_style = Style::default();
    let mut heading_level: Option<u32> = None;
    let code_style = Style::new().fg(theme.code).add_modifier(Modifier::DIM);
    let quote_style = Style::new().fg(theme.quote);
    let mut in_table = false;
    let mut in_table_head = false;
    let mut table_columns: usize = 0;
    let mut table_header: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    let flush_line = |lines: &mut Vec<Line<'static>>, current: &mut Vec<Span<'static>>| {
        if !current.is_empty() {
            lines.push(Line::from(std::mem::take(current)));
        }
    };

    let push_blank = |lines: &mut Vec<Line<'static>>| {
        if lines.last().map(|line| !line.spans.is_empty()).unwrap_or(false) {
            lines.push(Line::raw(""));
        }
    };

    for event in parser {
        if in_table {
            match event {
                MdEvent::Start(Tag::Table(alignments)) => {
                    table_columns = alignments.len();
                }
                MdEvent::End(TagEnd::Table) => {
                    flush_line(&mut lines, &mut current);
                    render_table(
                        &mut lines,
                        &table_header,
                        &table_rows,
                        table_width,
                        table_columns,
                    );
                    lines.push(Line::raw(""));
                    in_table = false;
                    in_table_head = false;
                    table_columns = 0;
                    table_header.clear();
                    table_rows.clear();
                    current_row.clear();
                    current_cell.clear();
                }
                MdEvent::Start(Tag::TableHead) => {
                    in_table_head = true;
                    table_header.clear();
                    current_row.clear();
                }
                MdEvent::End(TagEnd::TableHead) => {
                    if !current_cell.is_empty() {
                        current_row.push(current_cell.trim().to_string());
                        current_cell.clear();
                    }
                    if table_columns > 0 {
                        pad_row(&mut current_row, table_columns);
                    }
                    if table_header.is_empty() && !current_row.is_empty() {
                        table_header = current_row.clone();
                        current_row.clear();
                    }
                    in_table_head = false;
                }
                MdEvent::Start(Tag::TableRow) => {
                    current_row.clear();
                }
                MdEvent::End(TagEnd::TableRow) => {
                    if !current_cell.is_empty() {
                        current_row.push(current_cell.trim().to_string());
                        current_cell.clear();
                    }
                    if table_columns > 0 {
                        pad_row(&mut current_row, table_columns);
                    }
                    if in_table_head && table_header.is_empty() {
                        table_header = current_row.clone();
                    } else if !in_table_head {
                        table_rows.push(current_row.clone());
                    }
                    current_row.clear();
                }
                MdEvent::Start(Tag::TableCell) => {
                    current_cell.clear();
                }
                MdEvent::End(TagEnd::TableCell) => {
                    current_row.push(current_cell.trim().to_string());
                    current_cell.clear();
                }
                MdEvent::Text(text) => {
                    current_cell.push_str(&text);
                }
                MdEvent::Code(code) => {
                    if !current_cell.is_empty() {
                        current_cell.push(' ');
                    }
                    current_cell.push_str(&code);
                }
                MdEvent::SoftBreak => {
                    current_cell.push(' ');
                }
                MdEvent::HardBreak => {
                    current_cell.push(' ');
                }
                _ => {}
            }
            continue;
        }

        match event {
            MdEvent::Start(Tag::Table(alignments)) => {
                in_table = true;
                in_table_head = false;
                table_columns = alignments.len();
                table_header.clear();
                table_rows.clear();
                current_row.clear();
                current_cell.clear();
            }
            MdEvent::Start(Tag::Heading { level, .. }) => {
                push_blank(&mut lines);
                heading_level = Some(level as u32);
            }
            MdEvent::End(TagEnd::Heading(_)) => {
                flush_line(&mut lines, &mut current);
                lines.push(Line::raw(""));
                heading_level = None;
            }
            MdEvent::Start(Tag::Paragraph) => {}
            MdEvent::End(TagEnd::Paragraph) => {
                flush_line(&mut lines, &mut current);
                lines.push(Line::raw(""));
            }
            MdEvent::Start(Tag::List(_)) => {
                push_blank(&mut lines);
                list_depth += 1;
            }
            MdEvent::End(TagEnd::List(_)) => {
                if list_depth > 0 {
                    list_depth -= 1;
                }
                lines.push(Line::raw(""));
            }
            MdEvent::Start(Tag::Item) => {
                flush_line(&mut lines, &mut current);
                if list_depth > 0 {
                    current.push(Span::raw("  ".repeat(list_depth.saturating_sub(1))));
                }
                current.push(Span::styled("- ", Style::new().fg(theme.list_bullet)));
            }
            MdEvent::End(TagEnd::Item) => {
                flush_line(&mut lines, &mut current);
            }
            MdEvent::Start(Tag::BlockQuote) => {
                in_blockquote = true;
            }
            MdEvent::End(TagEnd::BlockQuote) => {
                in_blockquote = false;
                flush_line(&mut lines, &mut current);
                lines.push(Line::raw(""));
            }
            MdEvent::Start(Tag::Emphasis) => {
                style_stack.push(current_style);
                current_style = current_style.add_modifier(Modifier::ITALIC);
            }
            MdEvent::End(TagEnd::Emphasis) => {
                current_style = style_stack.pop().unwrap_or_default();
            }
            MdEvent::Start(Tag::Strong) => {
                style_stack.push(current_style);
                current_style = current_style.add_modifier(Modifier::BOLD);
            }
            MdEvent::End(TagEnd::Strong) => {
                current_style = style_stack.pop().unwrap_or_default();
            }
            MdEvent::Start(Tag::Strikethrough) => {
                style_stack.push(current_style);
                current_style = current_style.add_modifier(Modifier::CROSSED_OUT);
            }
            MdEvent::End(TagEnd::Strikethrough) => {
                current_style = style_stack.pop().unwrap_or_default();
            }
            MdEvent::Start(Tag::CodeBlock(_)) => {
                flush_line(&mut lines, &mut current);
                push_blank(&mut lines);
                in_code_block = true;
            }
            MdEvent::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                lines.push(Line::raw(""));
            }
            MdEvent::Text(text) => {
                if in_code_block {
                    for line in text.split('\n') {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", line),
                            code_style,
                        )));
                    }
                } else {
                    if in_blockquote && current.is_empty() {
                        current.push(Span::styled("> ", quote_style));
                    }
                    let mut style = current_style;
                    if let Some(level) = heading_level {
                        style = style.add_modifier(Modifier::BOLD).fg(theme.heading);
                        if level <= 2 {
                            style = style.add_modifier(Modifier::UNDERLINED);
                        }
                    }
                    current.push(Span::styled(text.to_string(), style));
                }
            }
            MdEvent::Code(code) => {
                if in_blockquote && current.is_empty() {
                    current.push(Span::styled("> ", quote_style));
                }
                current.push(Span::styled(format!("`{}`", code), code_style));
            }
            MdEvent::SoftBreak => current.push(Span::raw(" ")),
            MdEvent::HardBreak => {
                flush_line(&mut lines, &mut current);
            }
            MdEvent::Rule => {
                push_blank(&mut lines);
                lines.push(Line::from(Span::styled(
                    "-".repeat(32),
                    Style::new().fg(theme.rule),
                )));
                lines.push(Line::raw(""));
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current);
    lines
}

pub fn estimate_rendered_lines(lines: &[Line<'static>], width: u16) -> u16 {
    if width == 0 {
        return 0;
    }
    let width = width as usize;
    let mut total: usize = 0;
    for line in lines {
        let line_width = line.width().max(1);
        let wrapped = (line_width + width - 1) / width;
        total = total.saturating_add(wrapped.max(1));
    }
    total.min(u16::MAX as usize) as u16
}

fn render_table(
    lines: &mut Vec<Line<'static>>,
    header: &[String],
    rows: &[Vec<String>],
    max_width: u16,
    col_hint: usize,
) {
    let mut col_count = col_hint.max(header.len());
    for row in rows {
        col_count = col_count.max(row.len());
    }
    if col_count == 0 {
        return;
    }

    let mut widths = vec![0usize; col_count];
    for (idx, cell) in header.iter().enumerate() {
        widths[idx] = widths[idx].max(cell.chars().count());
    }
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.chars().count());
        }
    }

    let widths = fit_table_widths(widths, col_count, max_width);

    let format_wrapped_row = |row: &[String]| -> Vec<String> {
        let mut wrapped_cells: Vec<Vec<String>> = Vec::with_capacity(col_count);
        let mut row_height = 1usize;
        for i in 0..col_count {
            let cell = row.get(i).map(String::as_str).unwrap_or("");
            let wrapped = wrap_cell(cell, widths[i].max(1));
            row_height = row_height.max(wrapped.len());
            wrapped_cells.push(wrapped);
        }

        let mut output: Vec<String> = Vec::with_capacity(row_height);
        for line_idx in 0..row_height {
            let mut out = String::from("|");
            for i in 0..col_count {
                let cell_line = wrapped_cells[i].get(line_idx).map(String::as_str).unwrap_or("");
                let pad = widths[i].saturating_sub(cell_line.chars().count());
                out.push(' ');
                out.push_str(cell_line);
                out.push_str(&" ".repeat(pad));
                out.push(' ');
                out.push('|');
            }
            output.push(out);
        }
        output
    };

    if !header.is_empty() {
        for line in format_wrapped_row(header) {
            lines.push(Line::raw(line));
        }
        let mut sep = String::from("|");
        for w in &widths {
            sep.push_str(" ");
            sep.push_str(&"-".repeat((*w).max(1)));
            sep.push_str(" |");
        }
        lines.push(Line::raw(sep));
    }

    for row in rows {
        for line in format_wrapped_row(row) {
            lines.push(Line::raw(line));
        }
    }
}

fn fit_table_widths(widths: Vec<usize>, col_count: usize, max_width: u16) -> Vec<usize> {
    if max_width == 0 || col_count == 0 {
        return widths;
    }
    let max_width = max_width as usize;
    let total_overhead = 1 + col_count * 3;
    if total_overhead >= max_width {
        return vec![1; col_count];
    }
    let available = max_width - total_overhead;
    let desired_sum: usize = widths.iter().sum();
    if desired_sum <= available {
        return widths;
    }

    let mut new_widths: Vec<usize> = widths
        .iter()
        .map(|w| ((w * available) / desired_sum).max(1))
        .collect();
    let mut used: usize = new_widths.iter().sum();
    let mut remaining = available.saturating_sub(used);
    while remaining > 0 {
        let mut best_idx = 0;
        let mut best_need = 0usize;
        for (idx, (&want, &have)) in widths.iter().zip(new_widths.iter()).enumerate() {
            let need = want.saturating_sub(have);
            if need > best_need {
                best_need = need;
                best_idx = idx;
            }
        }
        if best_need == 0 {
            break;
        }
        new_widths[best_idx] += 1;
        used += 1;
        remaining = available.saturating_sub(used);
    }

    new_widths
}

fn wrap_cell(cell: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    for ch in cell.chars() {
        if current_len == width {
            lines.push(std::mem::take(&mut current));
            current_len = 0;
        }
        current.push(ch);
        current_len += 1;
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn pad_row(row: &mut Vec<String>, columns: usize) {
    if row.len() >= columns {
        return;
    }
    row.extend(std::iter::repeat(String::new()).take(columns - row.len()));
}
