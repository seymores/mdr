use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::theme::Theme;

pub fn apply_beeline(lines: &[Line<'static>], theme: &Theme) -> Vec<Line<'static>> {
    lines
        .iter()
        .enumerate()
        .map(|(idx, line)| apply_beeline_line(line, idx, theme))
        .collect()
}

fn apply_beeline_line(line: &Line<'static>, index: usize, theme: &Theme) -> Line<'static> {
    let total_len = line
        .spans
        .iter()
        .map(|span| span.content.chars().count())
        .sum::<usize>();

    if total_len == 0 {
        return line.clone();
    }

    let forward = index.is_multiple_of(2);
    let mut pos = 0usize;
    let mut spans: Vec<Span<'static>> = Vec::new();

    for span in &line.spans {
        for ch in span.content.chars() {
            let t = if total_len <= 1 {
                0.0
            } else {
                pos as f32 / (total_len - 1) as f32
            };
            let t = if forward { t } else { 1.0 - t };
            let mut style = span.style;
            if style.fg.is_none() {
                style.fg = Some(lerp_color(theme.beeline_start, theme.beeline_end, t));
            }
            spans.push(Span::styled(ch.to_string(), style));
            pos += 1;
        }
    }

    Line {
        spans,
        style: line.style,
        alignment: line.alignment,
    }
}

fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    match (start, end) {
        (Color::Rgb(sr, sg, sb), Color::Rgb(er, eg, eb)) => {
            let r = lerp_u8(sr, er, t);
            let g = lerp_u8(sg, eg, t);
            let b = lerp_u8(sb, eb, t);
            Color::Rgb(r, g, b)
        }
        _ => start,
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    let v = a + (b - a) * t.clamp(0.0, 1.0);
    v.round().clamp(0.0, 255.0) as u8
}
