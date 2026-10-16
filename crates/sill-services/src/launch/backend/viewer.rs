//! Handing a file to the viewer: one call on `org.quire.Anyview1` over a connection to the session
//! bus, in the shape the viewer's own platform crate writes (`anyview_platform::Request`). The bus
//! starts the viewer for the call when it is installed with its activation file; when the call
//! fails (no viewer on this machine, no session bus), the caller opens the file with the default
//! handler as it did before the viewer existed.
//!
//! The place the pane held is `Resume::Nothing`: the pane shows a file's start and keeps no place
//! of its own, so the viewer continues where the person last left the file.

use std::path::Path;

use anyview_core::{FilePath, NonEmpty, ResultsId, Resume, Sequence, SequenceOrigin};
use anyview_platform::linux::forward_over;
use anyview_platform::{Handoff, Request};
use sill_launcher::{OpenTarget, ViewerOpen};

use crate::deadline::{BUS_CALL, within};
use crate::env::Bus;

/// Whether the viewer took the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Handed {
    Yes,
    No,
}

/// The request `target` makes of the viewer, when it is one that the viewer takes and every path
/// in it is one a request can name (absolute and UTF-8).
pub fn viewer_request(target: &OpenTarget) -> Option<Request> {
    match target {
        OpenTarget::Viewer(open) => handoff(open),
        OpenTarget::Play(file) => FilePath::new(file).ok().map(Request::Play),
        OpenTarget::File(_) | OpenTarget::Reveal(_) | OpenTarget::Url(_) => None,
    }
}

/// `open` as a handoff: its results are the viewer's sequence when they hold the file, and no
/// sequence otherwise (the viewer then lists the file's folder).
fn handoff(open: &ViewerOpen) -> Option<Request> {
    let file = FilePath::new(&open.file).ok()?;
    let entries: Vec<FilePath> = open
        .results
        .iter()
        .filter_map(|path| FilePath::new(Path::new(path)).ok())
        .collect();
    let sequence = NonEmpty::from_vec(entries).and_then(|entries| {
        Sequence::starting_at(
            entries,
            &file,
            SequenceOrigin::Results(ResultsId(open.search.0)),
        )
        .ok()
    });
    Some(Request::Handoff(Handoff {
        file,
        resume: Resume::Nothing,
        sequence,
    }))
}

/// Call the viewer with `request` on `session`.
pub(crate) async fn hand_over(session: &Bus, request: &Request) -> Handed {
    let connection = match session.connect().await {
        Ok(Some(connection)) => connection,
        Ok(None) | Err(_) => return Handed::No,
    };
    match within(BUS_CALL, "the viewer", forward_over(&connection, request)).await {
        Ok(Ok(())) => Handed::Yes,
        Ok(Err(_)) | Err(_) => Handed::No,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyview_core::{FilePath, ResultsId, Resume, SequenceOrigin};
    use anyview_platform::Request;
    use sill_launcher::{Generation, OpenTarget, ViewerOpen};

    use super::viewer_request;

    fn open(file: &str, results: &[&str]) -> OpenTarget {
        OpenTarget::Viewer(ViewerOpen {
            file: PathBuf::from(file),
            search: Generation(41),
            results: results.iter().map(PathBuf::from).collect(),
        })
    }

    fn handed(target: &OpenTarget) -> anyview_platform::Handoff {
        match viewer_request(target) {
            Some(Request::Handoff(handoff)) => handoff,
            other => panic!("not a handoff: {other:?}"),
        }
    }

    #[test]
    fn the_results_are_the_viewers_sequence_pointing_at_the_file() {
        let handoff = handed(&open("/h/b.pdf", &["/h/a.pdf", "/h/b.pdf", "/h/c.png"]));
        assert_eq!(
            handoff.file,
            FilePath::new("/h/b.pdf").unwrap_or_else(|e| panic!("{e}"))
        );
        assert_eq!(handoff.resume, Resume::Nothing);
        let sequence = handoff.sequence.unwrap_or_else(|| panic!("no sequence"));
        let names: Vec<&str> = sequence
            .entries()
            .iter()
            .map(|entry| entry.as_path().to_str().unwrap_or_default())
            .collect();
        assert_eq!(names, ["/h/a.pdf", "/h/b.pdf", "/h/c.png"]);
        assert_eq!(sequence.current().as_path(), PathBuf::from("/h/b.pdf"));
        assert_eq!(sequence.origin(), &SequenceOrigin::Results(ResultsId(41)));
    }

    #[test]
    fn a_file_outside_its_results_or_with_none_goes_without_a_sequence() {
        let cases = [
            ("no results", open("/h/a.pdf", &[])),
            ("results without the file", open("/h/a.pdf", &["/h/b.pdf"])),
        ];
        for (name, target) in cases {
            assert_eq!(handed(&target).sequence, None, "{name}");
        }
    }

    #[test]
    fn only_the_viewers_targets_are_requests_and_only_with_absolute_paths() {
        let play = FilePath::new("/h/a.flac").unwrap_or_else(|e| panic!("{e}"));
        let cases = [
            (
                "a play",
                OpenTarget::Play(PathBuf::from("/h/a.flac")),
                Some(Request::Play(play)),
            ),
            ("a relative file", open("a.pdf", &[]), None),
            (
                "a relative play",
                OpenTarget::Play(PathBuf::from("a.flac")),
                None,
            ),
            (
                "the default handler",
                OpenTarget::File(PathBuf::from("/h/a.pdf")),
                None,
            ),
            (
                "a reveal",
                OpenTarget::Reveal(PathBuf::from("/h/a.pdf")),
                None,
            ),
            (
                "an address",
                OpenTarget::Url("https://example.com".to_owned()),
                None,
            ),
        ];
        for (name, target, want) in cases {
            assert_eq!(viewer_request(&target), want, "{name}");
        }
    }
}
