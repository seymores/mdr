# mdr

A small TUI markdown reader for the terminal.

## Features
- Terminal UI with a pastel color theme.
- Keyboard navigation: Up/Down, PageUp/PageDown, Home/End, and j/k.
- Mouse wheel scrolling.
- Basic markdown styling for headings, lists, emphasis, inline code, blockquotes, and rules.
- Tables with column fitting and multi-line cell wrapping.
- Scrollbar that hides when all content fits on screen.

## Screenshot
Add a screenshot of the UI here.

```text
screenshot: docs/screenshot.png
```

## Usage
```bash
cargo run -- path/to/file.md
```

## Notes
- The UI is intentionally lightweight; rendering is plain-text with styling rather than full layout.
- Tables wrap long cells vertically to fit the current viewport width.
