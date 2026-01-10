use super::*;

#[test]
fn wrap_ranges_and_link_hit_test() {
    let text = String::from("abcd efghij");
    let (wraps, offsets) = build_wraps(&[text.clone()], 5);
    let lines_text = vec![text.clone()];
    let links = vec![LinkTarget {
        line_idx: 0,
        start_char: 2,
        end_char: 8,
        url: "https://example.com".to_string(),
    }];

    let hit_row0 = link_at_position(&links, &wraps, &offsets, &lines_text, 0, 2);
    assert!(hit_row0.is_some());

    let hit_row1 = link_at_position(&links, &wraps, &offsets, &lines_text, 1, 1);
    assert!(hit_row1.is_some());

    let miss = link_at_position(&links, &wraps, &offsets, &lines_text, 1, 4);
    assert!(miss.is_none());
}

#[test]
fn search_matches_scroll_to_wrapped_row() {
    let lines = vec![String::from("hello world")];
    let (wraps, offsets) = build_wraps(&lines, 5);
    let matches = build_search_matches(&lines, "world", &wraps, &offsets, 5);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].scroll_pos, 2);
}

#[test]
fn esc_closes_help_without_overwriting_scroll_restore() {
    let mut state = AppState::new(true);
    state.scroll = 12;
    state.show_help = true;
    state.scroll_before_help = Some(12);

    let action = state.handle_key_input(KeyCode::Esc, 100, 10);

    assert!(matches!(action, KeyAction::None));
    assert!(!state.show_help);
    assert_eq!(state.scroll, 12);
    assert_eq!(state.scroll_before_help, Some(12));
}

#[test]
fn enter_without_search_requests_link_open() {
    let mut state = AppState::new(true);
    state.search_mode = false;
    state.show_help = false;

    let action = state.handle_key_input(KeyCode::Enter, 100, 10);

    assert!(matches!(action, KeyAction::OpenLink));
}

#[test]
fn help_toggle_records_scroll_and_clears_hover() {
    let mut state = AppState::new(true);
    state.scroll = 7;
    state.hover_link = Some("https://example.com".to_string());

    let action = state.handle_key_input(KeyCode::Char('h'), 100, 10);

    assert!(matches!(action, KeyAction::None));
    assert!(state.show_help);
    assert_eq!(state.scroll_before_help, Some(7));
    assert!(state.hover_link.is_none());
}

#[test]
fn search_enter_jumps_to_first_match() {
    let mut state = AppState::new(true);
    state.search_mode = true;
    state.search_matches = vec![SearchMatch {
        line_idx: 0,
        start: 0,
        end: 4,
        start_char: 0,
        scroll_pos: 9,
    }];

    let action = state.handle_key_input(KeyCode::Enter, 100, 10);

    assert!(matches!(action, KeyAction::None));
    assert!(!state.search_mode);
    assert_eq!(state.scroll, 9);
}
