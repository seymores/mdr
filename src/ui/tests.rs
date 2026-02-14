use super::*;

#[test]
fn wrap_ranges_and_link_hit_test() {
    let text = String::from("abcd efghij");
    let (wraps, offsets) = build_wraps(std::slice::from_ref(&text), 5);
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

#[test]
fn bracket_keys_emit_document_navigation_actions() {
    let mut state = AppState::new(true);

    let next = state.handle_key_input(KeyCode::Char(']'), 100, 10);
    let prev = state.handle_key_input(KeyCode::Char('['), 100, 10);

    assert!(matches!(next, KeyAction::NextDocument));
    assert!(matches!(prev, KeyAction::PreviousDocument));
}

#[test]
fn g_key_emits_go_dialog_action() {
    let mut state = AppState::new(true);
    let action = state.handle_key_input(KeyCode::Char('g'), 100, 10);
    assert!(matches!(action, KeyAction::OpenGoDialog));
}

#[test]
fn queue_label_shows_current_position_and_filename() {
    let label = queue_label(1, 4, "docs/guide.md");
    assert_eq!(label, "[2/4] docs/guide.md");
}

#[test]
fn switching_documents_resets_hover_and_search_mode() {
    let mut state = AppState::new(true);
    state.hover_link = Some("https://example.com".to_string());
    state.search_mode = true;
    state.search_query = "needle".to_string();
    state.search_matches = vec![SearchMatch {
        line_idx: 0,
        start: 0,
        end: 6,
        start_char: 0,
        scroll_pos: 2,
    }];

    state.on_document_changed();

    assert!(state.hover_link.is_none());
    assert!(!state.search_mode);
    assert!(state.search_query.is_empty());
    assert!(state.search_matches.is_empty());
}

#[test]
fn picker_escape_closes_overlay_without_queue_change() {
    let mut state = AppState::new(true);
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("a.md"), "# a").expect("write md");
    state.open_picker(root.path().to_path_buf());
    let selected_before = state.picker_selected;

    let result = state.handle_picker_key_input(KeyCode::Esc);

    assert!(matches!(result, EventResult::Continue));
    assert!(!state.picker_open);
    assert_eq!(selected_before, 0);
}

#[test]
fn picker_enter_on_directory_descends_into_it() {
    let mut state = AppState::new(true);
    let root = tempfile::tempdir().expect("tempdir");
    let child = root.path().join("child");
    std::fs::create_dir_all(&child).expect("create child");

    state.open_picker(root.path().to_path_buf());
    let dir_index = state
        .picker_entries
        .iter()
        .position(|entry| entry.path.file_name() == Some("child".as_ref()))
        .expect("directory entry should exist");
    state.picker_selected = dir_index;

    let result = state.handle_picker_key_input(KeyCode::Enter);

    assert!(matches!(result, EventResult::Continue));
    assert_eq!(state.picker_dir.file_name(), Some("child".as_ref()));
}

#[test]
fn picker_enter_on_markdown_file_returns_open_path() {
    let mut state = AppState::new(true);
    let root = tempfile::tempdir().expect("tempdir");
    let target = root.path().join("target.md");
    std::fs::write(&target, "# t").expect("write md");

    state.open_picker(root.path().to_path_buf());
    let file_index = state
        .picker_entries
        .iter()
        .position(|entry| entry.path.file_name() == Some("target.md".as_ref()))
        .expect("file entry should exist");
    state.picker_selected = file_index;

    let result = state.handle_picker_key_input(KeyCode::Enter);

    assert!(
        matches!(result, EventResult::OpenPath(path) if path.file_name() == Some("target.md".as_ref()))
    );
}

#[test]
fn switching_documents_updates_queue_indicator() {
    assert_eq!(queue_label(0, 3, "a.md"), "[1/3] a.md");
    assert_eq!(queue_label(2, 3, "c.md"), "[3/3] c.md");
}

#[test]
fn help_lines_show_separate_previous_and_next_bindings() {
    let lines: Vec<String> = help_lines()
        .into_iter()
        .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect();

    assert!(
        lines
            .iter()
            .any(|line| line.contains("]                    Next document"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("[                    Previous document"))
    );
}

#[test]
fn go_dialog_enter_returns_selected_index() {
    let mut state = AppState::new(true);
    state.open_go_dialog(3, 1);

    let move_result = state.handle_go_dialog_key_input(KeyCode::Down);
    assert!(matches!(move_result, EventResult::Continue));

    let result = state.handle_go_dialog_key_input(KeyCode::Enter);
    assert!(matches!(result, EventResult::GoToIndex(2)));
    assert!(!state.go_dialog_open);
}
