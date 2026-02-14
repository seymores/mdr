# Multi-Document Queue And Filesystem Picker Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add queue-based multi-document reading plus filesystem markdown discovery/opening inside the TUI, including a clear visual indicator of the current file position in the queue, while preserving existing markdown rendering, search, link, and scrolling behavior.

**Architecture:** Split startup concerns into `cli` parsing and `file_discovery`, then introduce a `document_queue` domain model. Update the TUI to read from the active queue item, support queue navigation keys, and provide an in-app picker overlay that scans/filter markdown files and enqueues selections.

**Tech Stack:** Rust 2024, ratatui, crossterm, std::fs/std::path, `tempfile` (dev-dependency for deterministic filesystem tests).

---

## Brainstorming Outcome

Recommended approach: **hybrid discovery**.

1. Discover markdown files from CLI inputs (`file` or `directory`) at startup.
2. Add an in-app picker (`o`) for on-demand filesystem scanning/open.
3. Keep markdown render pipeline unchanged by swapping the active document content only.

Alternatives considered:

1. Pre-index entire filesystem at startup. Rejected because startup latency becomes unpredictable.
2. Shell out to `fd`/`fzf`. Rejected because it adds non-portable runtime dependencies.
3. Keep only CLI multi-file support without picker. Rejected because it does not satisfy “open and look for md files on fs” in-app.

## Guardrails

- Apply `@superpowers/test-driven-development` per task.
- Use small commits after each task.
- Before completion, apply `@superpowers/verification-before-completion`.
- Preserve backward compatibility for existing single-file usage and keybindings.

### Task 1: Parse Multi-Input CLI Arguments

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `src/cli.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn parses_no_beeline_and_multiple_inputs() {
    let parsed = parse_args(["mdr", "--no-beeline", "a.md", "docs"]).unwrap();
    assert!(!parsed.enable_beeline);
    assert_eq!(parsed.inputs, vec![PathBuf::from("a.md"), PathBuf::from("docs")]);
}

#[test]
fn returns_usage_error_when_no_inputs() {
    let err = parse_args(["mdr"]).unwrap_err();
    assert!(err.contains("Usage: mdr"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test cli::tests::parses_no_beeline_and_multiple_inputs -q`  
Expected: FAIL with unresolved `parse_args`/`CliArgs`.

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct CliArgs {
    pub enable_beeline: bool,
    pub inputs: Vec<PathBuf>,
}

pub fn parse_args<I, S>(args: I) -> Result<CliArgs, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    // Accept --no-beeline and N input paths.
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test cli::tests::parses_no_beeline_and_multiple_inputs -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: parse multiple markdown inputs from cli"
```

### Task 2: Add Filesystem Markdown Discovery

**Files:**
- Create: `src/file_discovery.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`
- Test: `src/file_discovery.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn expands_directories_and_filters_markdown_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("nested")).unwrap();
    fs::write(dir.path().join("a.md"), "# a").unwrap();
    fs::write(dir.path().join("nested/b.markdown"), "# b").unwrap();
    fs::write(dir.path().join("nested/c.txt"), "nope").unwrap();

    let found = discover_markdown_paths(&[dir.path().to_path_buf()]).unwrap();
    assert_eq!(found.len(), 2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test file_discovery::tests::expands_directories_and_filters_markdown_files -q`  
Expected: FAIL with missing module/function.

**Step 3: Write minimal implementation**

```rust
pub fn discover_markdown_paths(inputs: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    // Files pass through if extension is markdown.
    // Directories are walked recursively.
    // Return sorted, deduplicated absolute paths.
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test file_discovery::tests::expands_directories_and_filters_markdown_files -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/file_discovery.rs Cargo.toml src/main.rs
git commit -m "feat: discover markdown files from directories"
```

### Task 3: Introduce Document Queue Domain

**Files:**
- Create: `src/document_queue.rs`
- Modify: `src/main.rs`
- Test: `src/document_queue.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn next_and_prev_wrap_across_queue() {
    let docs = vec![
        QueuedDocument::new("a.md".into(), "a".into()),
        QueuedDocument::new("b.md".into(), "b".into()),
    ];
    let mut q = DocumentQueue::new(docs).unwrap();
    q.next();
    assert_eq!(q.current().path, PathBuf::from("b.md"));
    q.next();
    assert_eq!(q.current().path, PathBuf::from("a.md"));
    q.prev();
    assert_eq!(q.current().path, PathBuf::from("b.md"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test document_queue::tests::next_and_prev_wrap_across_queue -q`  
Expected: FAIL with missing queue types.

**Step 3: Write minimal implementation**

```rust
pub struct QueuedDocument {
    pub path: PathBuf,
    pub content: String,
}

pub struct DocumentQueue {
    docs: Vec<QueuedDocument>,
    current: usize,
}

impl DocumentQueue {
    pub fn current(&self) -> &QueuedDocument { /* ... */ }
    pub fn next(&mut self) { /* wrap */ }
    pub fn prev(&mut self) { /* wrap */ }
    pub fn push_and_focus(&mut self, doc: QueuedDocument) { /* ... */ }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test document_queue::tests::next_and_prev_wrap_across_queue -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/document_queue.rs src/main.rs
git commit -m "feat: add document queue model for multi-doc reading"
```

### Task 4: Wire Startup Pipeline Into Queue

**Files:**
- Modify: `src/main.rs`
- Modify: `src/cli.rs`
- Modify: `src/file_discovery.rs`
- Modify: `src/ui.rs`
- Test: `src/main.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn load_queue_from_mixed_file_and_directory_inputs() {
    // Build temporary filesystem with one file arg and one directory arg.
    // Assert the resulting queue length/order matches discovery rules.
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test main::tests::load_queue_from_mixed_file_and_directory_inputs -q`  
Expected: FAIL because loader helper and queue wiring do not exist.

**Step 3: Write minimal implementation**

```rust
fn load_initial_queue(args: CliArgs) -> Result<DocumentQueue, String> {
    // discover paths -> read files -> build queue
}

fn main() {
    // parse args, build queue, call ui::run_tui(queue, enable_beeline)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test main::tests::load_queue_from_mixed_file_and_directory_inputs -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/main.rs src/cli.rs src/file_discovery.rs src/ui.rs
git commit -m "feat: initialize ui with document queue from filesystem inputs"
```

### Task 5: Add Queue Navigation In TUI And Queue Indicator

**Files:**
- Modify: `src/ui.rs`
- Modify: `src/ui/tests.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn bracket_keys_emit_document_navigation_actions() {
    let mut state = AppState::new(true);
    let next = state.handle_key_input(KeyCode::Char(']'), 100, 10);
    let prev = state.handle_key_input(KeyCode::Char('['), 100, 10);
    assert!(matches!(next, KeyAction::NextDocument));
    assert!(matches!(prev, KeyAction::PreviousDocument));
}

#[test]
fn queue_label_shows_current_position_and_filename() {
    let label = queue_label(1, 4, "docs/guide.md");
    assert_eq!(label, "[2/4] docs/guide.md");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test ui::tests::bracket_keys_emit_document_navigation_actions -q`  
Expected: FAIL with missing `KeyAction` variants and key handling.

**Step 3: Write minimal implementation**

```rust
enum KeyAction {
    None,
    Quit,
    OpenLink,
    NextDocument,
    PreviousDocument,
    OpenPicker,
}

// In handle_key_input:
KeyCode::Char(']') => KeyAction::NextDocument,
KeyCode::Char('[') => KeyAction::PreviousDocument,
KeyCode::Char('o') => KeyAction::OpenPicker,
```

Add queue indicator rendering in the title/footer:

```rust
// Example title rendering
let title_text = queue_label(current_index, queue_len, current_path_str);
// => "[3/12] /path/to/current.md"
```

Behavior requirements:
- Always show queue position as `[current/total]` (1-based).
- Show current file path (or basename if full path does not fit).
- Update immediately when moving with `[` / `]` or opening from picker.
- Keep existing right-aligned status (scroll/search) intact.

**Step 4: Run test to verify it passes**

Run: `cargo test ui::tests::bracket_keys_emit_document_navigation_actions -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui.rs src/ui/tests.rs
git commit -m "feat: support queue navigation keys in tui"
```

### Task 6: Build In-App Filesystem Picker Overlay

**Files:**
- Create: `src/picker.rs`
- Modify: `src/main.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/tests.rs`
- Test: `src/picker.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn filter_paths_matches_case_insensitive_substring() {
    let items = vec![
        PathBuf::from("/tmp/guide.md"),
        PathBuf::from("/tmp/README.markdown"),
        PathBuf::from("/tmp/notes.txt"),
    ];
    let out = filter_paths(&items, "read");
    assert_eq!(out, vec![PathBuf::from("/tmp/README.markdown")]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test picker::tests::filter_paths_matches_case_insensitive_substring -q`  
Expected: FAIL with missing picker module/functions.

**Step 3: Write minimal implementation**

```rust
pub fn filter_paths(paths: &[PathBuf], query: &str) -> Vec<PathBuf> {
    // lowercase contains() filter over filename/full path
}
```

Add picker mode to `AppState`:

```rust
picker_open: bool,
picker_query: String,
picker_results: Vec<PathBuf>,
picker_selected: usize,
picker_scan_root: PathBuf,
```

Render a centered popup with query + result list; `Enter` opens selected markdown, enqueues with `push_and_focus`, and returns to reader view.

**Step 4: Run test to verify it passes**

Run: `cargo test picker::tests::filter_paths_matches_case_insensitive_substring -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/picker.rs src/ui.rs src/ui/tests.rs src/main.rs
git commit -m "feat: add markdown picker overlay for filesystem open"
```

### Task 7: Regression Coverage, Help Text, And Verification

**Files:**
- Modify: `src/ui/tests.rs`
- Modify: `README.md`
- Create: `tests/queue_startup.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn startup_with_directory_input_loads_markdown_queue() {
    // Integration test over helper API: directory -> queue len > 0
}
```

Also add UI behavior tests:

```rust
#[test]
fn switching_documents_resets_hover_and_search_mode() { /* ... */ }

#[test]
fn picker_escape_closes_overlay_without_queue_change() { /* ... */ }

#[test]
fn switching_documents_updates_queue_indicator() { /* ... */ }
```

**Step 2: Run test to verify it fails**

Run: `cargo test queue_startup -q`  
Expected: FAIL until startup helper and UI transitions are fully wired.

**Step 3: Write minimal implementation**

- Finish missing wiring for queue switch state reset.
- Update help screen with:
  - `[` / `]` previous/next document
  - `o` open markdown picker
- Update usage/docs for:
  - `cargo run -- file1.md dir2`
  - queue navigation and picker behavior.

**Step 4: Run full verification**

Run: `cargo fmt --check`  
Expected: PASS.

Run: `cargo test`  
Expected: PASS.

Run: `cargo clippy --all-targets --all-features -- -D warnings`  
Expected: PASS.

**Step 5: Commit**

```bash
git add README.md src/ui.rs src/ui/tests.rs tests/queue_startup.rs
git commit -m "test: cover queue startup and picker flows"
```

## Definition Of Done

- CLI accepts multiple markdown paths and directories.
- Directory inputs recursively discover markdown files.
- Reader launches with a queue, not a single document.
- In-reader keys `[` and `]` navigate queue items.
- UI always shows queue indicator in format `[current/total] current-file`.
- Key `o` opens picker overlay, filters markdown files, and opens selected file into queue.
- Existing search/link/help/scroll behavior continues to pass regression tests.
- README documents new behavior and keybindings.
