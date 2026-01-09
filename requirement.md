# Requirements And Prompt Steps

## Requirements
- Rust TUI markdown reader named `mdr`.
- Terminal UI with pastel color theme.
- Keyboard navigation: Up/Down, PageUp/PageDown, Home/End, and j/k.
- Mouse wheel scrolling.
- Scrollbar that hides when content fits the viewport.
- Markdown styling: headings, lists, emphasis, inline code, blockquotes, rules.
- Table rendering with auto-fit to viewport width and wrapped cell content (no truncation).
- CLI usage: `cargo run -- path/to/file.md`.
- README with features, usage, and screenshot section.
- Placeholder screenshot at `docs/screenshot.png`.

## Prompt Steps (In Order)
1) "Start a new project, a TUI markdown reader, in rust language."
2) "Add TUI and markdown dependencies (ratatui, crossterm, pulldown-cmark) and scaffold the UI loop."
3) "Add styled markdown rendering (headings, code blocks, lists with spacing)."
4) "Try compile and fix errors."
5) "Add page-up/page-down and percentage scroll indicator."
6) "Add Home/End keys and a visible scroll bar."
7) "Fix scrolling so it works in iTerm2; keys register but content didn’t move."
8) "Hide the scrollbar if the page fits the screen; also hide status when no scrolling is needed."
9) "Fix scrollbar thumb reaching the end of the content."
10) "Add support for tables."
11) "Fix table width so it auto-fits screen width."
12) "Do not truncate table cells; wrap cell content to multiple lines."
13) "Fix missing table header."
14) "Fix missing third column in table rows by padding rows and flushing last cell."
15) "Add mouse wheel scroll support."
16) "Add pastel color theme."
17) "Create README with features and usage."
18) "Add screenshot section and placeholder at docs/screenshot.png."
