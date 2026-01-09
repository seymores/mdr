use std::env;
use std::fs;
use std::process;

mod beeline;
mod markdown;
mod theme;
mod ui;

fn main() {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Usage: mdr <path-to-markdown>");
            process::exit(2);
        }
    };

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read {}: {}", path, err);
            process::exit(1);
        }
    };

    if let Err(err) = ui::run_tui(&path, &content) {
        eprintln!("TUI error: {}", err);
        process::exit(1);
    }
}
