//! Tracker 3 (localsearch's `org.freedesktop.Tracker3.Miner.Files` endpoint) over zbus: one
//! SPARQL `Query` per ask, results read from the pipe the endpoint writes its cursor to. Lane a
//! fills it; [`sparql_for`] and [`escape_literal`] are pure and table-tested.
//!
//! The query matches `nfo:fileName` (and `fts:match` on the text when the miner indexes
//! content), keeps hits under `ask.under`, and asks for `nie:url`, `nie:mimeType` and
//! `nfo:fileLastModified`, ordered by rank, limited to `ask.limit`.

use sill_launcher::Query;

use super::index::{FileAnswer, FileAsk, FileIndex, IndexFault, IndexKind};

/// The Tracker 3 index. Holds no connection until the first ask.
#[derive(Debug, Default)]
pub struct TrackerIndex;

impl FileIndex for TrackerIndex {
    fn kind(&self) -> IndexKind {
        IndexKind::Tracker
    }

    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer {
        let _ = (ask, query);
        FileAnswer::Unavailable(IndexFault::NotRunning)
    }
}

/// The SPARQL for `ask`, with the text and the folder as escaped string literals.
pub fn sparql_for(ask: &FileAsk) -> String {
    let _ = ask;
    todo!("M9 fill: a-files")
}

/// `text` as the inside of a SPARQL `"…"` literal: backslash, both quotes, newline, carriage
/// return and tab escaped (SPARQL 1.1 §19.7 ECHAR), so a typed query cannot close the literal.
pub fn escape_literal(text: &str) -> String {
    let _ = text;
    todo!("M9 fill: a-files")
}

#[cfg(test)]
mod tests {
    use super::escape_literal;

    #[test]
    #[ignore = "M9 fill: a-files"]
    fn a_typed_query_cannot_leave_its_literal() {
        let cases: &[(&str, &str)] = &[
            ("notes", "notes"),
            (r#"a"b"#, r#"a\"b"#),
            ("a'b", r"a\'b"),
            (r"a\b", r"a\\b"),
            ("a\nb\tc\r", r"a\nb\tc\r"),
            (r#"x" } DROP"#, r#"x\" } DROP"#),
        ];
        for &(text, want) in cases {
            assert_eq!(escape_literal(text), want, "{text:?}");
        }
    }
}
