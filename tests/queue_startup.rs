#![allow(dead_code)]

#[path = "../src/document_queue.rs"]
mod document_queue;
#[path = "../src/file_discovery.rs"]
mod file_discovery;

use std::fs;

use document_queue::{DocumentQueue, QueuedDocument};
use file_discovery::discover_markdown_paths;

#[test]
fn startup_with_directory_input_loads_markdown_queue() {
    let root = tempfile::tempdir().expect("create tempdir");
    let docs = root.path().join("docs");
    fs::create_dir_all(&docs).expect("create docs");

    let first = docs.join("a.md");
    let second = docs.join("b.markdown");
    let ignored = docs.join("c.txt");

    fs::write(&first, "# first").expect("write first");
    fs::write(&second, "# second").expect("write second");
    fs::write(&ignored, "ignore").expect("write ignored");

    let discovered = discover_markdown_paths(&[docs]).expect("discover markdown files");
    assert_eq!(discovered.len(), 2);

    let queued: Vec<QueuedDocument> = discovered
        .into_iter()
        .map(|path| {
            let content = fs::read_to_string(&path).expect("read markdown");
            QueuedDocument::new(path, content)
        })
        .collect();

    let queue = DocumentQueue::new(queued).expect("create queue");
    assert_eq!(queue.len(), 2);
}
