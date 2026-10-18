//! A file index for tests: answers every ask with the hits whose name contains the text
//! (case-insensitively), from a list the test gives. It never reads the disk.

use sill_launcher::Query;

use super::index::{FileAnswer, FileAsk, FileHit, FileIndex};

/// A made-up index.
#[derive(Debug, Clone, Default)]
pub struct FakeIndex {
    pub hits: Vec<FileHit>,
}

impl FileIndex for FakeIndex {
    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer {
        let _ = query;
        let text = ask.text.to_lowercase();
        FileAnswer::Hits(
            self.hits
                .iter()
                .filter(|hit| hit.name.to_lowercase().contains(&text))
                .filter(|hit| hit.path.starts_with(&ask.under))
                .take(usize::from(ask.limit.0))
                .cloned()
                .collect(),
        )
    }
}
