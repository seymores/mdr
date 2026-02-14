use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedDocument {
    pub path: PathBuf,
    pub content: String,
}

impl QueuedDocument {
    pub fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}

#[derive(Clone, Debug)]
pub struct DocumentQueue {
    docs: Vec<QueuedDocument>,
    current: usize,
}

impl DocumentQueue {
    pub fn new(docs: Vec<QueuedDocument>) -> Result<Self, String> {
        if docs.is_empty() {
            return Err("Document queue cannot be empty".to_string());
        }
        Ok(Self { docs, current: 0 })
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    pub fn current(&self) -> &QueuedDocument {
        &self.docs[self.current]
    }

    pub fn documents(&self) -> &[QueuedDocument] {
        &self.docs
    }

    pub fn next(&mut self) {
        if self.docs.len() > 1 {
            self.current = (self.current + 1) % self.docs.len();
        }
    }

    pub fn prev(&mut self) {
        if self.docs.len() > 1 {
            self.current = if self.current == 0 {
                self.docs.len() - 1
            } else {
                self.current - 1
            };
        }
    }

    pub fn push_and_focus(&mut self, doc: QueuedDocument) {
        self.docs.push(doc);
        self.current = self.docs.len() - 1;
    }

    pub fn focus_existing(&mut self, path: &Path) -> bool {
        if let Some(idx) = self.docs.iter().position(|doc| doc.path == path) {
            self.current = idx;
            true
        } else {
            false
        }
    }

    pub fn focus_index(&mut self, idx: usize) -> bool {
        if idx < self.docs.len() {
            self.current = idx;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
