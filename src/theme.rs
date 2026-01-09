use ratatui::style::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub border: Color,
    pub title: Color,
    pub footer: Color,
    pub heading: Color,
    pub list_bullet: Color,
    pub code: Color,
    pub quote: Color,
    pub rule: Color,
    pub scrollbar_thumb: Color,
    pub scrollbar_track: Color,
    pub beeline_start: Color,
    pub beeline_end: Color,
    pub search_bg: Color,
    pub search_fg: Color,
    pub search_bg_active: Color,
    pub search_fg_active: Color,
}

impl Theme {
    pub fn pastel() -> Self {
        Self {
            border: Color::Rgb(184, 193, 236),
            title: Color::Rgb(132, 140, 200),
            footer: Color::Rgb(160, 168, 210),
            heading: Color::Rgb(140, 180, 220),
            list_bullet: Color::Rgb(152, 210, 190),
            code: Color::Rgb(240, 200, 170),
            quote: Color::Rgb(190, 170, 220),
            rule: Color::Rgb(190, 190, 200),
            scrollbar_thumb: Color::Rgb(150, 190, 220),
            scrollbar_track: Color::Rgb(210, 220, 230),
            beeline_start: Color::Rgb(170, 200, 230),
            beeline_end: Color::Rgb(230, 170, 200),
            search_bg: Color::Rgb(255, 230, 170),
            search_fg: Color::Rgb(60, 60, 60),
            search_bg_active: Color::Rgb(255, 200, 120),
            search_fg_active: Color::Rgb(40, 40, 40),
        }
    }
}
