use super::*;
use crate::theme::Theme;

fn line_text(line: &Line<'static>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn renders_basic_markdown_blocks() {
    let md = r#"# Title

- item

```
code
```

| A | B |
| - | - |
| 1 | 2 |
"#;
    let theme = Theme::pastel();
    let lines = render_markdown_with_links(md, 80, &theme).0;
    let text: Vec<String> = lines.iter().map(line_text).collect();

    assert!(text.iter().any(|line| line == "Title"));
    assert!(text.iter().any(|line| line.contains("- item")));
    assert!(text.iter().any(|line| line.contains("    code")));
    assert!(text.iter().any(|line| line.contains("| A | B |")));
    assert!(text.iter().any(|line| line.contains("| 1 | 2 |")));
}
