//! Tracker 3 (localsearch's `org.freedesktop.Tracker3.Miner.Files` endpoint) over zbus: one
//! SPARQL `Query` per ask, results read from the pipe the endpoint writes its cursor to. Lane a
//! fills it; [`sparql_for`] and [`escape_literal`] are pure and table-tested.
//!
//! The query matches `nfo:fileName` (and `fts:match` on the text when the miner indexes
//! content), keeps hits under `ask.under`, and asks for `nie:url`, `nie:mimeType` and
//! `nfo:fileLastModified`, ordered by rank, limited to `ask.limit`.
//!
//! As filled (FINDINGS "M9 lane a"): the name alone is matched (a case-insensitive substring,
//! as `fd` does), never `fts:match`, because a row must say why it is there and `hits.rs` drops
//! a hit whose name does not match; with no rank to order by, the newest come first.

use std::collections::HashMap;
use std::io::{PipeReader, PipeWriter, Read};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};
use sill_launcher::Query;
use zbus::Connection;
use zbus::zvariant::{Fd, Value};

use super::bus::{NameState, block_on, name_state};
use super::choice::IndexState;
use super::cursor::{hit_of_row, parse_cursor};
use super::index::{FileAnswer, FileAsk, FileIndex, IndexFault, IndexKind};

/// The miner's bus name (localsearch keeps it beside its newer `org.freedesktop.LocalSearch3`).
const TRACKER_BUS: &str = "org.freedesktop.Tracker3.Miner.Files";
const ENDPOINT_PATH: &str = "/org/freedesktop/Tracker3/Endpoint";
const ENDPOINT_INTERFACE: &str = "org.freedesktop.Tracker3.Endpoint";

/// How often the pipe's reader looks at the clock and the query's generation.
const TICK: Duration = Duration::from_millis(10);

/// What a path keeps unescaped in a `file:` URI: GLib's `g_filename_to_uri` set (unreserved
/// characters, the sub-delimiters, `:`, `@` and `/`), so a prefix compares equal to the
/// `nie:url` Tracker stored.
const URI_PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'!')
    .remove(b'$')
    .remove(b'&')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'+')
    .remove(b',')
    .remove(b';')
    .remove(b'=')
    .remove(b':')
    .remove(b'@')
    .remove(b'/');

/// The Tracker 3 index. Holds no connection until the first ask.
#[derive(Debug, Default)]
pub struct TrackerIndex;

impl FileIndex for TrackerIndex {
    fn kind(&self) -> IndexKind {
        IndexKind::Tracker
    }

    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer {
        if query.is_cancelled() {
            return FileAnswer::Unavailable(IndexFault::Cancelled);
        }
        let deadline = Instant::now() + ask.budget;
        let Ok((reader, writer)) = std::io::pipe() else {
            return FileAnswer::Unavailable(IndexFault::Failed);
        };
        let Some(chunks) = read_on_a_thread(reader) else {
            return FileAnswer::Unavailable(IndexFault::Failed);
        };
        let sparql = sparql_for(ask);
        let called = block_on(tokio::time::timeout(
            ask.budget,
            ask_endpoint(&sparql, &writer),
        ));
        drop(writer);
        match called {
            None => FileAnswer::Unavailable(IndexFault::NoBus),
            Some(Err(_)) => FileAnswer::Partial(Vec::new()),
            Some(Ok(Err(fault))) => FileAnswer::Unavailable(fault),
            Some(Ok(Ok(()))) => collect(&chunks, deadline, query, &ask.under),
        }
    }
}

/// Whether the miner is running (has an owner) or only activatable; asks the bus only, never
/// the index, and starts nothing.
pub(super) fn tracker_state() -> IndexState {
    block_on(miner_state())
        .and_then(Result::ok)
        .unwrap_or(IndexState::Absent)
}

/// The SPARQL for `ask`, with the text and the folder as escaped string literals.
pub fn sparql_for(ask: &FileAsk) -> String {
    let prefix = format!("{}/", file_uri(&ask.under).trim_end_matches('/'));
    format!(
        "SELECT DISTINCT ?url ?mime ?modified WHERE {{ \
         GRAPH tracker:FileSystem {{ \
         ?file a nfo:FileDataObject ; nfo:fileName ?name ; nie:url ?url ; \
         nfo:fileLastModified ?modified . \
         FILTER (STRSTARTS(?url, \"{prefix}\") && CONTAINS(LCASE(?name), \"{text}\")) }} \
         OPTIONAL {{ ?file nie:interpretedAs ?content . ?content nie:mimeType ?mime }} }} \
         ORDER BY DESC(?modified) LIMIT {limit}",
        prefix = escape_literal(&prefix),
        text = escape_literal(&ask.text.to_lowercase()),
        limit = ask.limit.0,
    )
}

/// `text` as the inside of a SPARQL `"…"` literal: backslash, both quotes, newline, carriage
/// return and tab escaped (SPARQL 1.1 §19.7 ECHAR), so a typed query cannot close the literal.
pub fn escape_literal(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut out, c| {
            match c {
                '\\' => out.push_str(r"\\"),
                '"' => out.push_str("\\\""),
                '\'' => out.push_str(r"\'"),
                '\n' => out.push_str(r"\n"),
                '\r' => out.push_str(r"\r"),
                '\t' => out.push_str(r"\t"),
                other => out.push(other),
            }
            out
        })
}

/// `path` as a `file://` URI, percent-encoded byte by byte.
pub(crate) fn file_uri(path: &Path) -> String {
    format!(
        "file://{}",
        percent_encode(path.as_os_str().as_bytes(), URI_PATH)
    )
}

async fn miner_state() -> zbus::Result<IndexState> {
    let connection = Connection::session().await?;
    Ok(match name_state(&connection, TRACKER_BUS).await? {
        NameState::Owned => IndexState::Running,
        NameState::Activatable => IndexState::Dormant,
        NameState::Missing => IndexState::Absent,
    })
}

/// `Endpoint.Query(sparql, the pipe's write end, {})`; the cursor arrives on the pipe.
async fn ask_endpoint(sparql: &str, writer: &PipeWriter) -> Result<(), IndexFault> {
    let connection = Connection::session().await.map_err(|_| IndexFault::NoBus)?;
    let arguments: HashMap<&str, Value<'_>> = HashMap::new();
    connection
        .call_method(
            Some(TRACKER_BUS),
            ENDPOINT_PATH,
            Some(ENDPOINT_INTERFACE),
            "Query",
            &(sparql, Fd::from(writer), arguments),
        )
        .await
        .map(|_| ())
        .map_err(|error| fault_of(&error))
}

fn fault_of(error: &zbus::Error) -> IndexFault {
    match error {
        zbus::Error::MethodError(name, _, _)
            if name.as_str().ends_with(".ServiceUnknown")
                || name.as_str().ends_with(".NameHasNoOwner") =>
        {
            IndexFault::NotRunning
        }
        _ => IndexFault::Failed,
    }
}

/// The pipe's bytes, chunk by chunk, until the endpoint closes it.
fn read_on_a_thread(mut reader: PipeReader) -> Option<Receiver<Vec<u8>>> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("sill-tracker-read".to_owned())
        .spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        })
        .ok()?;
    Some(rx)
}

/// Read the cursor until it ends (`Hits`), the deadline passes (`Partial`) or a newer
/// keystroke cancels the query.
fn collect(
    chunks: &Receiver<Vec<u8>>,
    deadline: Instant,
    query: &Query,
    under: &Path,
) -> FileAnswer {
    let mut bytes = Vec::new();
    let finished = loop {
        if query.is_cancelled() {
            return FileAnswer::Unavailable(IndexFault::Cancelled);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break Finished::OutOfTime;
        }
        match chunks.recv_timeout(left.min(TICK)) {
            Ok(chunk) => bytes.extend(chunk),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break Finished::Whole,
        }
    };
    let hits = parse_cursor(&bytes)
        .iter()
        .filter_map(|row| hit_of_row(row, under))
        .collect();
    match finished {
        Finished::Whole => FileAnswer::Hits(hits),
        Finished::OutOfTime => FileAnswer::Partial(hits),
    }
}

/// Whether the whole cursor was read.
enum Finished {
    Whole,
    OutOfTime,
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{escape_literal, file_uri, sparql_for};
    use crate::providers::files::index::{FileAsk, HitLimit};

    #[test]
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

    #[test]
    fn a_path_is_a_file_uri_as_glib_writes_it() {
        let cases: &[(&str, &str)] = &[
            ("/home/p", "file:///home/p"),
            ("/home/p/a b.pdf", "file:///home/p/a%20b.pdf"),
            ("/home/p/日", "file:///home/p/%E6%97%A5"),
            ("/home/p/a#b?c%", "file:///home/p/a%23b%3Fc%25"),
            ("/home/p/it's (1)+x@y", "file:///home/p/it's%20(1)+x@y"),
        ];
        for &(path, want) in cases {
            assert_eq!(file_uri(Path::new(path)), want, "{path}");
        }
    }

    #[test]
    fn the_query_holds_the_text_and_folder_as_literals_and_the_limit() {
        let cases: &[(&str, &str, &str)] = &[
            ("Report", "/home/p", "CONTAINS(LCASE(?name), \"report\")"),
            (
                "x\") } DROP",
                "/home/p",
                "CONTAINS(LCASE(?name), \"x\\\") } drop\")",
            ),
            (
                "a",
                "/home/my dir/",
                "STRSTARTS(?url, \"file:///home/my%20dir/\")",
            ),
            ("a", "/home/p", "STRSTARTS(?url, \"file:///home/p/\")"),
        ];
        for &(text, under, want) in cases {
            let sparql = sparql_for(&FileAsk {
                text: text.to_owned(),
                under: PathBuf::from(under),
                limit: HitLimit(40),
                budget: Duration::from_millis(150),
            });
            assert!(sparql.contains(want), "{text:?} under {under}: {sparql}");
            assert!(sparql.ends_with("LIMIT 40"), "{sparql}");
        }
    }
}
