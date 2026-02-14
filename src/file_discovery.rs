use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn discover_markdown_paths(inputs: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut discovered = Vec::new();

    for input in inputs {
        if input.is_dir() {
            walk_dir(input, &mut discovered)?;
            continue;
        }

        if input.is_file() && is_markdown(input) {
            let absolute = fs::canonicalize(input).unwrap_or_else(|_| input.clone());
            discovered.push(absolute);
            continue;
        }

        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path not found or unsupported: {}", input.display()),
        ));
    }

    discovered.sort();
    discovered.dedup();
    Ok(discovered)
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out)?;
        } else if path.is_file() && is_markdown(&path) {
            let absolute = fs::canonicalize(&path).unwrap_or(path.clone());
            out.push(absolute);
        }
    }

    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            matches!(ext.as_str(), "md" | "markdown" | "mdown" | "mdx")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
