//! Hits to rows, the same for every index. Pure; lane a fills it.
//!
//! A row's title is the file name, its subtitle the folder with the home directory as `~`
//! (`~/Documents/Taxes`), its icon the MIME type's glyph name (`IconRef::Glyph`: `folder`,
//! `image`, `file`), its subject `Subject::File`, its id the absolute path, its actions
//! "Reveal in Files" (⌘R) and "Copy Path" (⌘C). The score is `rank_text` over the name with the
//! row's frecency, plus a recency boost for a file modified in the last week (proposed: +50,
//! +25 within a month), so a document you are working on outranks an old namesake.
//!
//! A hit whose name does not match is dropped, content hits included (lane a's call, FINDINGS
//! "M9 lane a"): the row shows a name, so the name must be why it is there. Two indexes naming
//! one path make one row. Names match without case, as Spotlight's do (Q310).

use std::collections::BTreeSet;
use std::path::Path;

use nucleo_matcher::Matcher;
use sill_launcher::{
    Action, ActionId, ActionKeys, FileFacts, Frecency, IconRef, Item, ItemId, ItemKey, Mime,
    Needle, ProviderKind, RankWeights, Score, Subject, UnixTime, chords, rank_text,
};

use super::index::FileHit;

/// The action ids a file row carries.
pub const REVEAL_ACTION: &str = "reveal";
pub const COPY_PATH_ACTION: &str = "copy-path";

/// The recency boost: `(younger than this many days, points)`, first match wins.
const RECENT: [(u64, u32); 2] = [(7, 50), (30, 25)];

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
    let needle = without_case(needle);
    let mut seen = BTreeSet::new();
    let mut rows: Vec<Item> = hits
        .iter()
        .filter(|hit| seen.insert(hit.path.clone()))
        .filter_map(|hit| file_item(hit, &needle, home, frecency, now, weights, matcher))
        .collect();
    rows.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.0.cmp(&b.id.0))
    });
    rows
}

/// `needle` matching without case, as Spotlight matches file names (Q310): the launcher's
/// needle is smart case, and a lower-case needle is case-insensitive. nucleo's operators
/// (`^`, `$`, `'`, `!`) have no case, so lower-casing keeps them.
fn without_case(needle: &Needle) -> Needle {
    Needle::parse(&needle.text().to_lowercase())
}

/// `folder` as a row shows it: under `home` as `~/…`, else as it is.
pub fn tilde(folder: &Path, home: &Path) -> String {
    match folder.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => folder.display().to_string(),
    }
}

fn file_item(
    hit: &FileHit,
    needle: &Needle,
    home: &Path,
    frecency: &Frecency,
    now: UnixTime,
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Option<Item> {
    let id = ItemId(hit.path.to_string_lossy().into_owned());
    let used = frecency.score(&ItemKey::of(ProviderKind::Files, &id), now);
    let Score(matched) = rank_text(needle, &hit.name, used, weights, matcher)?;
    let folder = hit.path.parent().unwrap_or(&hit.path);
    Some(Item {
        id,
        provider: ProviderKind::Files,
        title: hit.name.clone(),
        subtitle: tilde(folder, home),
        icon: IconRef::Glyph(glyph_of(&hit.mime).to_owned()),
        score: Score(matched + recency_boost(hit.modified, now)),
        actions: file_actions(),
        subject: Subject::File(FileFacts {
            path: hit.path.clone(),
            mime: hit.mime.clone(),
            modified: hit.modified,
        }),
    })
}

/// Points for a file modified lately (a file dated in the future counts as today's).
fn recency_boost(modified: UnixTime, now: UnixTime) -> u32 {
    let age = now.days_since(modified);
    RECENT
        .iter()
        .find(|(days, _)| age < *days)
        .map_or(0, |(_, points)| *points)
}

fn glyph_of(mime: &Mime) -> &'static str {
    match mime.0.as_str() {
        Mime::FOLDER => "folder",
        other if other.starts_with("image/") => "image",
        _ => "file",
    }
}

fn file_actions() -> Vec<Action> {
    vec![
        Action {
            id: ActionId(REVEAL_ACTION.to_owned()),
            label: "Reveal in Files".to_owned(),
            keys: ActionKeys::Chord(chords::reveal()),
        },
        Action {
            id: ActionId(COPY_PATH_ACTION.to_owned()),
            label: "Copy Path".to_owned(),
            keys: ActionKeys::Chord(chords::copy()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use sill_launcher::{
        ActionKeys, Frecency, IconRef, ItemId, ItemKey, Mime, Needle, ProviderKind, RankWeights,
        Subject, UnixTime, chords, matching::matcher,
    };

    use super::{file_items, recency_boost, tilde};
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

    fn rows_for(hits: &[FileHit], text: &str, frecency: &Frecency) -> Vec<sill_launcher::Item> {
        file_items(
            hits,
            &Needle::parse(text),
            Path::new("/home/p"),
            frecency,
            NOW,
            &RankWeights::DEFAULT,
            &mut matcher(),
        )
    }

    #[test]
    fn home_is_a_tilde() {
        let home = Path::new("/home/p");
        assert_eq!(tilde(Path::new("/home/p/Documents"), home), "~/Documents");
        assert_eq!(tilde(Path::new("/home/p"), home), "~");
        assert_eq!(tilde(Path::new("/srv/x"), home), "/srv/x");
    }

    #[test]
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

    #[test]
    fn the_boost_falls_with_age() {
        let cases: &[(u64, u32)] = &[(0, 50), (6, 50), (7, 25), (29, 25), (30, 0), (400, 0)];
        for &(days, want) in cases {
            let modified = UnixTime(NOW.0 - days * 86_400);
            assert_eq!(recency_boost(modified, NOW), want, "{days} days");
        }
        assert_eq!(
            recency_boost(UnixTime(NOW.0 + 86_400), NOW),
            50,
            "the future"
        );
    }

    #[test]
    fn a_row_has_its_glyph_id_and_chords() {
        let hits = vec![
            hit("/home/p/m9 dir", "inode/directory", 90),
            hit("/home/p/m9.png", "image/png", 90),
            hit("/home/p/m9.pdf", "application/pdf", 90),
        ];
        let rows = rows_for(&hits, "m9", &Frecency::default());
        let glyph = |id: &str| {
            rows.iter()
                .find(|r| r.id == ItemId(id.to_owned()))
                .map(|r| r.icon.clone())
        };
        let cases: &[(&str, &str)] = &[
            ("/home/p/m9 dir", "folder"),
            ("/home/p/m9.png", "image"),
            ("/home/p/m9.pdf", "file"),
        ];
        for &(id, want) in cases {
            assert_eq!(glyph(id), Some(IconRef::Glyph(want.to_owned())), "{id}");
        }
        let keys: Vec<&ActionKeys> = rows[0].actions.iter().map(|a| &a.keys).collect();
        assert_eq!(
            keys,
            [
                &ActionKeys::Chord(chords::reveal()),
                &ActionKeys::Chord(chords::copy())
            ]
        );
    }

    #[test]
    fn names_match_without_case() {
        let hits = vec![
            hit("/home/p/Documents/m9-report.pdf", "application/pdf", 90),
            hit("/home/p/Taxes/2025 RETURN.pdf", "application/pdf", 90),
        ];
        let cases: &[(&str, &[&str])] = &[
            ("M9", &["m9-report.pdf"]),
            ("m9-REP", &["m9-report.pdf"]),
            ("return", &["2025 RETURN.pdf"]),
            ("Return", &["2025 RETURN.pdf"]),
            ("^M9", &["m9-report.pdf"]),
            ("x9", &[]),
        ];
        for &(text, want) in cases {
            let rows = rows_for(&hits, text, &Frecency::default());
            let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
            assert_eq!(titles, want, "{text}");
        }
    }

    #[test]
    fn a_file_chosen_before_ranks_higher_and_a_path_is_one_row() {
        let hits = vec![
            hit("/home/p/a/notes.md", "text/markdown", 90),
            hit("/home/p/b/notes.md", "text/markdown", 90),
            hit("/home/p/b/notes.md", "text/markdown", 90),
        ];
        let key = ItemKey::of(
            ProviderKind::Files,
            &ItemId("/home/p/b/notes.md".to_owned()),
        );
        let frecency = Frecency::default().record(key, NOW);
        let rows = rows_for(&hits, "notes", &frecency);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
        assert_eq!(ids, ["/home/p/b/notes.md", "/home/p/a/notes.md"]);
    }
}
