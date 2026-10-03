//! What the viewer remembers, as the files provider's rows: `anyview_store::read_history` is the
//! recently viewed files, newest first. A viewed file ranks with the launcher's own frecency (each
//! view counts as one use, at the time of the view), and a viewed file the index did not bring
//! is a row of its own when its name matches, so a file read yesterday is found by its name
//! wherever it lives. The history is read, not owned: the viewer writes it.

use std::path::Path;
use std::time::UNIX_EPOCH;

use anyview_core::mime_for;
use anyview_store::{HistoryEntry, ResumeLabel};
use sill_launcher::{Frecency, ItemId, ItemKey, Mime, ProviderKind, UnixTime};

use super::index::FileHit;

/// A view within this many seconds after the launcher's own choice of the file is the same use:
/// the launcher opened it in the viewer.
const SAME_USE_SECS: u64 = 120;

/// `frecency` with each of `views` counted as a use of its file at the time it was viewed, unless
/// the launcher already counted a choice of that file just before (it opened it).
pub fn fold_views(frecency: Frecency, views: &[HistoryEntry]) -> Frecency {
    views.iter().fold(frecency, |counted, view| {
        let key = key_of(view.path.as_path());
        let viewed = UnixTime(view.viewed.0);
        let chosen_just_before = counted.entries.get(&key).is_some_and(|used| {
            viewed.0.saturating_sub(used.last.0) <= SAME_USE_SECS && used.last <= viewed
        });
        if chosen_just_before {
            counted
        } else {
            counted.record(key, viewed)
        }
    })
}

fn key_of(path: &Path) -> ItemKey {
    ItemKey::of(
        ProviderKind::Files,
        &ItemId(path.to_string_lossy().into_owned()),
    )
}

/// The viewed files as index hits, newest first: each whose name `wanted` accepts and that
/// `modified_of` still finds (its modification time; `None` for one that is gone), with the media
/// type of its kind. The names are asked first, so the disk is asked about the few that match.
pub fn viewed_hits(
    views: &[HistoryEntry],
    mut wanted: impl FnMut(&str) -> bool,
    modified_of: impl Fn(&Path) -> Option<UnixTime>,
) -> Vec<FileHit> {
    views
        .iter()
        .filter_map(|view| {
            let path = view.path.as_path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            if !wanted(&name) {
                return None;
            }
            Some(FileHit {
                name,
                path: path.to_path_buf(),
                mime: Mime(mime_for(view.kind).as_str().to_owned()),
                modified: modified_of(path)?,
            })
        })
        .collect()
}

/// The modification time of the file at `path`, or `None` when it cannot be read.
pub fn modified_of(path: &Path) -> Option<UnixTime> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let since = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(UnixTime(since.as_secs()))
}

/// `rows` with the place each viewed file was left after its folder ("~/Documents · Page 143"),
/// for the rows that are viewed files.
pub fn with_places(
    rows: Vec<sill_launcher::Item>,
    views: &[HistoryEntry],
) -> Vec<sill_launcher::Item> {
    rows.into_iter()
        .map(|mut row| {
            let place = place_of(&row.id, views);
            if let Some(place) = place {
                row.subtitle = format!("{} · {place}", row.subtitle);
            }
            row
        })
        .collect()
}

/// The words for where `id`'s file was left, when it was viewed and left anywhere worth saying.
fn place_of(id: &ItemId, views: &[HistoryEntry]) -> Option<String> {
    let view = views
        .iter()
        .find(|view| view.path.as_path() == Path::new(&id.0))?;
    match view.label {
        ResumeLabel::Unlabelled => None,
        label
        @ (ResumeLabel::Page { .. } | ResumeLabel::Time { .. } | ResumeLabel::Line { .. }) => {
            Some(label.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use anyview_core::{FilePath, FormatKind};
    use anyview_store::Viewed;
    use sill_launcher::{FrecencyScore, ItemId, ProviderKind};

    use super::*;

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn view(path: &str, kind: FormatKind, ago: u64, label: ResumeLabel) -> HistoryEntry {
        HistoryEntry {
            path: FilePath::new(path).unwrap_or_else(|e| panic!("{e}")),
            kind,
            viewed: Viewed(NOW.0 - ago),
            label,
        }
    }

    fn score(frecency: &Frecency, path: &str) -> FrecencyScore {
        frecency.score(&key_of(Path::new(path)), NOW)
    }

    #[test]
    fn a_view_counts_as_a_use_at_its_time() {
        let views = [
            view(
                "/h/a.pdf",
                FormatKind::Pdf,
                10 * 86_400,
                ResumeLabel::Unlabelled,
            ),
            view("/h/b.pdf", FormatKind::Pdf, 100, ResumeLabel::Unlabelled),
        ];
        let folded = fold_views(Frecency::default(), &views);
        assert_eq!(
            score(&folded, "/h/a.pdf"),
            FrecencyScore(70),
            "ten days old: the 5-14 day bucket, once"
        );
        assert_eq!(score(&folded, "/h/b.pdf"), FrecencyScore(100), "just now");
        assert_eq!(score(&folded, "/h/never.pdf"), FrecencyScore::NONE);
    }

    #[test]
    fn a_view_the_launcher_opened_is_not_counted_twice() {
        let chosen_at = UnixTime(NOW.0 - 3_600);
        let launcher = Frecency::default().record(key_of(Path::new("/h/a.pdf")), chosen_at);
        let cases = [
            ("seconds after the choice", 3_600 - 5, FrecencyScore(100)),
            ("an hour after the choice", 0, FrecencyScore(200)),
        ];
        for (name, ago, want) in cases {
            let views = [view(
                "/h/a.pdf",
                FormatKind::Pdf,
                ago,
                ResumeLabel::Unlabelled,
            )];
            let folded = fold_views(launcher.clone(), &views);
            assert_eq!(score(&folded, "/h/a.pdf"), want, "{name}");
        }
    }

    #[test]
    fn a_viewed_file_is_a_hit_with_its_kinds_media_type_unless_it_is_gone() {
        let views = [
            view("/h/a.pdf", FormatKind::Pdf, 5, ResumeLabel::Unlabelled),
            view(
                "/h/gone.png",
                FormatKind::Raster,
                6,
                ResumeLabel::Unlabelled,
            ),
            view(
                "/h/song.flac",
                FormatKind::Audio,
                7,
                ResumeLabel::Unlabelled,
            ),
        ];
        let mut asked = Vec::new();
        let hits = viewed_hits(
            &views,
            |name| {
                asked.push(name.to_owned());
                name != "ignored.txt"
            },
            |path| (path != Path::new("/h/gone.png")).then_some(UnixTime(42)),
        );
        assert_eq!(
            asked,
            ["a.pdf", "gone.png", "song.flac"],
            "every name is asked"
        );
        let seen: Vec<(&str, &str, u64)> = hits
            .iter()
            .map(|hit| (hit.name.as_str(), hit.mime.0.as_str(), hit.modified.0))
            .collect();
        assert_eq!(
            seen,
            [
                ("a.pdf", "application/pdf", 42),
                ("song.flac", "audio/mpeg", 42)
            ]
        );
        let none = viewed_hits(
            &views,
            |_| false,
            |_| panic!("the disk is not asked about a name that does not match"),
        );
        assert!(none.is_empty());
    }

    #[test]
    fn a_viewed_row_says_where_the_file_was_left() {
        use sill_launcher::{IconRef, Item, Score, Subject};
        let row = |path: &str| Item {
            id: ItemId(path.to_owned()),
            provider: ProviderKind::Files,
            title: String::new(),
            subtitle: "~/Documents".to_owned(),
            icon: IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: Subject::None,
        };
        let views = [
            view(
                "/h/a.pdf",
                FormatKind::Pdf,
                5,
                ResumeLabel::Page { number: 143 },
            ),
            view("/h/b.png", FormatKind::Raster, 5, ResumeLabel::Unlabelled),
        ];
        let rows = with_places(
            vec![row("/h/a.pdf"), row("/h/b.png"), row("/h/c.txt")],
            &views,
        );
        let subtitles: Vec<&str> = rows.iter().map(|r| r.subtitle.as_str()).collect();
        assert_eq!(
            subtitles,
            ["~/Documents · Page 143", "~/Documents", "~/Documents"]
        );
    }
}
