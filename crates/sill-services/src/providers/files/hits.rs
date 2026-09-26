//! Hits to rows, the same for every index. Pure; lane a fills it.
//!
//! A row's title is the file name, its subtitle the folder with the home directory as `~`
//! (`~/Documents/Taxes`), its icon the MIME type's glyph name (`IconRef::Glyph`: `folder`,
//! `image`, `file`), its subject `Subject::File`, its id the absolute path, its actions
//! "Reveal in Files" (⌘R) and "Copy Path" (⌘C). The score is `rank_text` over the name with the
//! row's frecency, plus a recency boost for a file modified in the last week (proposed: +50,
//! +25 within a month), so a document you are working on outranks an old namesake.

use std::path::Path;

use nucleo_matcher::Matcher;
use sill_launcher::{Frecency, Item, Needle, RankWeights, UnixTime};

use super::index::FileHit;

/// The action ids a file row carries.
pub const REVEAL_ACTION: &str = "reveal";
pub const COPY_PATH_ACTION: &str = "copy-path";

/// `hits` as rows for `needle`, best first; a hit whose name does not match is dropped (an
/// index can match content, but the row must say why it is there: content hits keep a floor
/// score instead, lane a's call, recorded in FINDINGS).
pub fn file_items(
    hits: &[FileHit],
    needle: &Needle,
    home: &Path,
    frecency: &Frecency,
    now: UnixTime,
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Vec<Item> {
    let _ = (hits, needle, home, frecency, now, weights, matcher);
    todo!("M9 fill: a-files")
}

/// `folder` as a row shows it: under `home` as `~/…`, else as it is.
pub fn tilde(folder: &Path, home: &Path) -> String {
    let _ = (folder, home);
    todo!("M9 fill: a-files")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use sill_launcher::{
        Frecency, Mime, Needle, RankWeights, Subject, UnixTime, matching::matcher,
    };

    use super::{file_items, tilde};
    use crate::providers::files::index::FileHit;

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn hit(path: &str, mime: &str, days_old: u64) -> FileHit {
        let path = PathBuf::from(path);
        FileHit {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path,
            mime: Mime(mime.to_owned()),
            modified: UnixTime(NOW.0 - days_old * 86_400),
        }
    }

    #[test]
    #[ignore = "M9 fill: a-files"]
    fn home_is_a_tilde() {
        let home = Path::new("/home/p");
        assert_eq!(tilde(Path::new("/home/p/Documents"), home), "~/Documents");
        assert_eq!(tilde(Path::new("/home/p"), home), "~");
        assert_eq!(tilde(Path::new("/srv/x"), home), "/srv/x");
    }

    #[test]
    #[ignore = "M9 fill: a-files"]
    fn a_recent_file_outranks_an_old_namesake_and_rows_carry_their_facts() {
        let hits = vec![
            hit("/home/p/old/report.pdf", "application/pdf", 400),
            hit("/home/p/Documents/report.pdf", "application/pdf", 1),
            hit("/home/p/unrelated.txt", "text/plain", 1),
        ];
        let rows = file_items(
            &hits,
            &Needle::parse("report"),
            Path::new("/home/p"),
            &Frecency::default(),
            NOW,
            &RankWeights::DEFAULT,
            &mut matcher(),
        );
        let subtitles: Vec<&str> = rows.iter().map(|r| r.subtitle.as_str()).collect();
        assert_eq!(subtitles, ["~/Documents", "~/old"]);
        assert!(
            matches!(&rows[0].subject, Subject::File(f) if f.path == Path::new("/home/p/Documents/report.pdf"))
        );
        assert_eq!(rows[0].actions.len(), 2);
    }
}
