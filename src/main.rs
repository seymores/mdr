use std::env;
use std::fs;
use std::process;

mod beeline;
mod markdown;
mod theme;
mod ui;

fn main() {
    let mut path: Option<String> = None;
    let mut enable_beeline = true;

    for arg in env::args().skip(1) {
        if arg == "--no-beeline" {
            enable_beeline = false;
        } else if path.is_none() {
            path = Some(arg);
        }
    }

    let path = match path {
        Some(path) => path,
        None => {
            eprintln!("Usage: mdr [--no-beeline] <path-to-markdown>");
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

    if let Err(err) = ui::run_tui(&path, &content, enable_beeline) {
        eprintln!("TUI error: {}", err);
        process::exit(1);
    }
}
