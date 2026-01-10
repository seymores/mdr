Repository Requirements (mdr)

1) Build a Rust TUI markdown reader named mdr.
2) Use ratatui + crossterm for terminal UI and input handling.
3) Load a markdown file from CLI path; show usage when missing.
4) Render markdown with headings, lists, emphasis, inline code, blockquotes, rules.
5) Support tables with headers and auto-fit columns to screen width; wrap long cells vertically.
6) Render code blocks with syntax highlighting; generic blocks should still be readable.
7) Provide pastel color theme; ensure headers are more prominent/bold.
8) Implement BeeLine reader gradient for line tracking; add --no-beeline flag; add runtime toggle with 'b'.
9) Add plain mode toggle with 'm' to disable BeeLine/styling without restart.
10) Scrolling: Up/Down, Space and Tab for page down, Backtab for page up; remove j/k; keep Home/End optional.
11) Mouse wheel scroll support.
12) Scrollbar: hide when content fits; correct thumb size/position at end.
13) Search: '/' to enter search, Enter to jump to first match, Esc to clear, n/N next/prev.
14) Highlight all matches subtly; highlight current match strongly.
15) Search navigation should scroll to correct wrapped line.
16) Help overlay: 'h' to toggle; show commands; closing help keeps current scroll position.
17) Footer hints: minimal; show search prompt when active; show link on hover; keep status right-aligned.
18) Link styling: underline + distinct color; show URL when hovering the exact link text only.
19) Open link on Enter; do not show link list.
20) Hover behavior: works immediately on startup in iTerm2; add mouse-capture priming workaround.
21) Link hit-testing should follow wrapped layout (best-effort is acceptable).
22) Add support for mouse click to open link.
23) Add support for mouse hover to show URL only over actual link text.
24) Fix bug: links hover should work without needing to move mouse out/in.
25) Fix bug: help overlay should not reset scroll to top when closed.
26) Add support for table header display and ensure all columns render.
27) Ensure table widths adapt to screen width without truncation.
28) Add search clear on Esc (clears highlights and state).
29) Add BeeLine toggle and plain mode toggle hints in help (not footer).
30) Add quick help menu on 'h'; only bottom hint should mention 'h' for help and 'q' to quit.
31) Add page down with Space and Tab.
32) Implement link hover and mouse selection without breaking text selection behavior.
33) Add tests: markdown rendering, wrapping/link hit-testing, search scroll position, help/scroll state, key behaviors.
34) Refactor code into smaller files (ui, markdown, theme, beeline) and keep logic maintainable.
35) Provide README with features, usage, key bindings, and install instructions.
36) Provide AGENTS.md contributor guide and requirement.txt reproduction steps.

Notes for LLMs
- Favor ASCII output in files.
- Use apply_patch for small edits.
- Keep the UI responsive; avoid expensive per-frame work.
