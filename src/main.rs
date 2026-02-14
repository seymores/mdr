use std::env;
use std::fs;
use std::process;

mod beeline;
mod cli;
mod document_queue;
mod file_discovery;
mod markdown;
mod picker;
mod theme;
mod ui;

use cli::parse_args;
use document_queue::{DocumentQueue, QueuedDocument};
use file_discovery::discover_markdown_paths;

fn main() {
    let args = match parse_args(env::args()) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(2);
        }
    };

    let enable_beeline = args.enable_beeline;
    let picker_root = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let queue = match load_initial_queue(&args.inputs) {
        Ok(queue) => queue,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
    };

    if let Err(err) = ui::run_tui(queue, picker_root, enable_beeline) {
        eprintln!("TUI error: {}", err);
        process::exit(1);
    }
}

fn load_initial_queue(inputs: &[std::path::PathBuf]) -> Result<DocumentQueue, String> {
    let paths = discover_markdown_paths(inputs)
        .map_err(|err| format!("Failed to discover markdown files: {}", err))?;

    if paths.is_empty() {
        return Err("No markdown files found from provided inputs".to_string());
    }

    let mut docs = Vec::with_capacity(paths.len());
    for path in paths {
        let content = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read {}: {}", path.display(), err))?;
        docs.push(QueuedDocument::new(path, content));
    }

    DocumentQueue::new(docs)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    #[test]
    fn load_queue_from_mixed_file_and_directory_inputs() {
        let root = tempfile::tempdir().expect("create tempdir");
        let single = root.path().join("z_single.md");
        let nested_dir = root.path().join("docs");
        let nested = nested_dir.join("a_nested.markdown");
        let ignored = nested_dir.join("ignored.txt");

        fs::create_dir_all(&nested_dir).expect("create nested dir");
        fs::write(&single, "# single").expect("write single");
        fs::write(&nested, "# nested").expect("write nested");
        fs::write(&ignored, "not markdown").expect("write ignored");

        let inputs = vec![single.clone(), nested_dir.clone()];
        let queue = load_initial_queue(&inputs).expect("queue should load");
        assert_eq!(queue.len(), 2);
        let first = queue.current().path.clone();
        assert!(first.ends_with("a_nested.markdown"));
    }
}
