use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerEntryKind {
    Parent,
    Directory,
    MarkdownFile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerEntry {
    pub path: PathBuf,
    pub kind: PickerEntryKind,
    pub label: String,
}

pub fn list_entries(dir: PathBuf, query: &str) -> io::Result<Vec<PickerEntry>> {
    let dir = fs::canonicalize(&dir).unwrap_or(dir);
    if !dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Not a directory: {}", dir.display()),
        ));
    }

    let query = query.trim().to_ascii_lowercase();
    let mut entries: Vec<PickerEntry> = Vec::new();

    if let Some(parent) = dir.parent() {
        entries.push(PickerEntry {
            path: parent.to_path_buf(),
            kind: PickerEntryKind::Parent,
            label: "../".to_string(),
        });
    }

    let mut listed: Vec<PickerEntry> = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if !query.is_empty() && !name.to_ascii_lowercase().contains(&query) {
            continue;
        }

        if path.is_dir() {
            listed.push(PickerEntry {
                path,
                kind: PickerEntryKind::Directory,
                label: format!("{}/", name),
            });
            continue;
        }

        if path.is_file() && is_markdown(&path) {
            listed.push(PickerEntry {
                path,
                kind: PickerEntryKind::MarkdownFile,
                label: name,
            });
        }
    }

    listed.sort_by(|a, b| {
        let a_kind = kind_order(&a.kind);
        let b_kind = kind_order(&b.kind);
        a_kind.cmp(&b_kind).then_with(|| {
            a.label
                .to_ascii_lowercase()
                .cmp(&b.label.to_ascii_lowercase())
        })
    });

    entries.extend(listed);
    Ok(entries)
}

fn kind_order(kind: &PickerEntryKind) -> u8 {
    match kind {
        PickerEntryKind::Parent => 0,
        PickerEntryKind::Directory => 1,
        PickerEntryKind::MarkdownFile => 2,
    }
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
    fn list_entries_includes_parent_dir_and_markdown_only_files() {
        let root = tempfile::tempdir().expect("temp dir");
        let nested = root.path().join("nested");
        std::fs::create_dir_all(&nested).expect("create nested");
        std::fs::write(root.path().join("a.md"), "# a").expect("write md");
        std::fs::write(root.path().join("b.txt"), "b").expect("write txt");

        let entries = list_entries(root.path().to_path_buf(), "").expect("list entries");

        assert!(entries.iter().any(|e| e.kind == PickerEntryKind::Parent));
        assert!(entries.iter().any(|e| {
            e.kind == PickerEntryKind::Directory && e.path.file_name() == Some("nested".as_ref())
        }));
        assert!(entries.iter().any(|e| {
            e.kind == PickerEntryKind::MarkdownFile && e.path.file_name() == Some("a.md".as_ref())
        }));
        assert!(
            !entries
                .iter()
                .any(|e| e.path.file_name() == Some("b.txt".as_ref()))
        );
    }

    #[test]
    fn list_entries_filters_by_query_case_insensitively() {
        let root = tempfile::tempdir().expect("temp dir");
        std::fs::write(root.path().join("README.markdown"), "# readme").expect("write markdown");
        std::fs::write(root.path().join("guide.md"), "# guide").expect("write markdown");

        let entries = list_entries(root.path().to_path_buf(), "read").expect("list entries");

        assert!(entries.iter().any(|e| {
            e.kind == PickerEntryKind::MarkdownFile
                && e.path.file_name() == Some("README.markdown".as_ref())
        }));
        assert!(!entries.iter().any(|e| {
            e.kind == PickerEntryKind::MarkdownFile
                && e.path.file_name() == Some("guide.md".as_ref())
        }));
    }
}
