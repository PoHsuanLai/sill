//! A path as a `file://` URI: what the file manager is asked to show and what Tracker stores.

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};

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

/// `path` as a `file://` URI, percent-encoded byte by byte.
pub fn file_uri(path: &Path) -> String {
    format!(
        "file://{}",
        percent_encode(path.as_os_str().as_bytes(), URI_PATH)
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::file_uri;

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
}
