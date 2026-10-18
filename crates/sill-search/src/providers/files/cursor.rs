//! The cursor Tracker 3's `Endpoint.Query` writes into the pipe it is handed, read back into
//! rows, and a row of `?url ?mime ?modified` read as a [`FileHit`].
//!
//! The stream is in the host's byte order, one row after another until the pipe closes: the
//! column count `n` (i32), `n` value types (i32; 0 is unbound, an OPTIONAL that matched
//! nothing), `n` end offsets (i32), then the row's strings, each followed by a NUL, `last
//! offset + 1` bytes in all. Column `i` runs from the byte after column `i - 1`'s end (0 for the
//! first) to its own end offset. Checked against a live localsearch in its test sandbox.

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use sill_launcher::{Mime, UnixTime};

use super::index::FileHit;
use super::stat::{Shape, hit_of, mime_of};

/// The value type Tracker writes for an unbound variable.
const UNBOUND: i32 = 0;

/// One row: a value per column, `None` where the variable was unbound.
pub(super) type Row = Vec<Option<String>>;

/// Every complete row in `bytes`; a row the budget cut short is dropped.
pub(super) fn parse_cursor(bytes: &[u8]) -> Vec<Row> {
    let mut reader = Reader { bytes, at: 0 };
    std::iter::from_fn(|| reader.row()).collect()
}

/// A row of `?url ?mime ?modified` as a hit under `under`, or `None` for anything else (not a
/// `file:` URL, outside `under`, the root).
pub(super) fn hit_of_row(row: &Row, under: &Path) -> Option<FileHit> {
    let [Some(url), mime, modified] = row.as_slice() else {
        return None;
    };
    let path = path_of_uri(url)?;
    if !path.starts_with(under) {
        return None;
    }
    let mime = match mime.as_deref() {
        Some(mime) if !mime.is_empty() => Mime(mime.to_owned()),
        _ => mime_of(&path, Shape::File),
    };
    let modified = modified.as_deref().map_or(UnixTime(0), unix_time);
    hit_of(path, mime, modified)
}

/// A `file://` URI's path, percent-decoded (bytes, not text: a Unix name need not be UTF-8).
pub(super) fn path_of_uri(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let bytes: Vec<u8> = percent_decode_str(encoded).collect();
    let path = PathBuf::from(OsString::from_vec(bytes));
    path.is_absolute().then_some(path)
}

/// An `xsd:dateTime` as whole seconds since the epoch; 0 when unreadable or before 1970.
fn unix_time(text: &str) -> UnixTime {
    let seconds = text
        .parse::<jiff::Timestamp>()
        .map_or(0, jiff::Timestamp::as_second);
    UnixTime(u64::try_from(seconds).unwrap_or(0))
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(len)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    fn int(&mut self) -> Option<i32> {
        let raw: [u8; 4] = self.take(4)?.try_into().ok()?;
        Some(i32::from_ne_bytes(raw))
    }

    fn ints(&mut self, count: usize) -> Option<Vec<i32>> {
        (0..count).map(|_| self.int()).collect()
    }

    fn row(&mut self) -> Option<Row> {
        let columns = usize::try_from(self.int()?).ok().filter(|n| *n > 0)?;
        let kinds = self.ints(columns)?;
        let ends = self.ints(columns)?;
        let last = usize::try_from(*ends.last()?).ok()?;
        let data = self.take(last.checked_add(1)?)?;
        let mut start = 0usize;
        let mut row = Vec::with_capacity(columns);
        for (kind, end) in kinds.iter().zip(&ends) {
            let end = usize::try_from(*end).ok()?;
            let text = data.get(start..end)?;
            row.push((*kind != UNBOUND).then(|| String::from_utf8_lossy(text).into_owned()));
            start = end + 1;
        }
        Some(row)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use sill_launcher::{Mime, UnixTime};

    use super::{Row, hit_of_row, parse_cursor, path_of_uri};

    /// One row as Tracker writes it: `(type, text)` per column.
    fn row_bytes(cells: &[(i32, &str)]) -> Vec<u8> {
        let mut out = Vec::new();
        let n = i32::try_from(cells.len()).unwrap_or(0);
        out.extend(n.to_ne_bytes());
        cells
            .iter()
            .for_each(|(kind, _)| out.extend(kind.to_ne_bytes()));
        let mut end = -1i32;
        for (_, text) in cells {
            end += 1 + i32::try_from(text.len()).unwrap_or(0);
            out.extend(end.to_ne_bytes());
        }
        for (_, text) in cells {
            out.extend(text.as_bytes());
            out.push(0);
        }
        out
    }

    fn cells(values: &[Option<&str>]) -> Row {
        values.iter().map(|v| v.map(str::to_owned)).collect()
    }

    #[test]
    fn rows_are_read_until_the_stream_ends_and_a_cut_row_is_dropped() {
        let first = row_bytes(&[(1, "file:///h/a.pdf"), (2, "application/pdf"), (5, "t")]);
        let second = row_bytes(&[(1, "file:///h/b"), (0, ""), (5, "")]);
        let mut stream = [first.clone(), second.clone()].concat();
        assert_eq!(
            parse_cursor(&stream),
            vec![
                cells(&[Some("file:///h/a.pdf"), Some("application/pdf"), Some("t")]),
                cells(&[Some("file:///h/b"), None, Some("")]),
            ]
        );
        stream.truncate(first.len() + second.len() - 3);
        assert_eq!(parse_cursor(&stream).len(), 1, "the cut second row");
        assert_eq!(parse_cursor(&[]), Vec::<Row>::new());
        assert_eq!(parse_cursor(&[0, 0, 0, 0]), Vec::<Row>::new());
    }

    #[test]
    fn a_file_uri_is_decoded_to_bytes() {
        let cases: &[(&str, Option<&str>)] = &[
            ("file:///home/p/a%20b.pdf", Some("/home/p/a b.pdf")),
            ("file:///home/p/%E6%97%A5.txt", Some("/home/p/日.txt")),
            ("https://example.org/a", None),
            ("file://relative", None),
        ];
        for &(uri, want) in cases {
            assert_eq!(path_of_uri(uri), want.map(PathBuf::from), "{uri}");
        }
    }

    #[test]
    fn a_row_is_a_hit_under_the_folder() {
        let under = Path::new("/home/p");
        let row = cells(&[
            Some("file:///home/p/m9%20dir"),
            Some("inode/directory"),
            Some("2026-09-26T15:14:14.909284Z"),
        ]);
        let hit = hit_of_row(&row, under).unwrap_or_else(|| panic!("no hit"));
        assert_eq!(hit.name, "m9 dir");
        assert_eq!(hit.mime, Mime("inode/directory".to_owned()));
        assert_eq!(hit.modified, UnixTime(1_790_435_654));

        let guessed = cells(&[Some("file:///home/p/a.png"), None, None]);
        let hit = hit_of_row(&guessed, under).unwrap_or_else(|| panic!("no hit"));
        assert_eq!(hit.mime, Mime("image/png".to_owned()));
        assert_eq!(hit.modified, UnixTime(0));

        let outside = cells(&[Some("file:///etc/passwd"), None, None]);
        assert_eq!(hit_of_row(&outside, under), None);
        assert_eq!(hit_of_row(&cells(&[Some("file:///home/p/a")]), under), None);
    }
}
