//! Dragging the thumbnail out as a file: quire's card says when a press crossed its drag
//! threshold (`ondrag`), and the thumbnail hands the file to shell-host's drag source
//! (`SurfaceHandle::start_drag`) while that press is still down: a `text/uri-list` of the
//! screenshot's `file://` URI, copy or move as the target chooses, with quire's `ShotGhost` as
//! the icon. The outcome comes back as a [`DragEnd`] for the machine: a copy keeps the file and
//! dismisses the card, a move counts the file as the target's, a cancel or a refusal leaves the
//! card and resumes the hold.

use shell_host::{DragAction, DragActions, DragOutcome, DragPayload, DragSpec};

use super::ghost::{GhostLook, ghost_icon};
use super::machine::DragEnd;
use super::picture::ThumbPicture;

/// What the machine makes of shell-host's outcome.
pub fn drag_end(outcome: &DragOutcome) -> DragEnd {
    match outcome {
        DragOutcome::Dropped {
            action: DragAction::Copy,
        } => DragEnd::Copied,
        DragOutcome::Dropped {
            action: DragAction::Move,
        } => DragEnd::Moved,
        DragOutcome::Cancelled => DragEnd::Cancelled,
        DragOutcome::Unsupported { .. } => DragEnd::Refused,
    }
}

/// The drag of `picture`'s file, its ghost drawn in `look`; `None` when the file's path makes no
/// URI (a relative path).
pub fn file_drag(picture: &ThumbPicture, look: GhostLook) -> Option<DragSpec> {
    let uri = ds::ImageSource::file(&picture.file).ok()?.0;
    Some(DragSpec {
        payload: DragPayload::Uris(vec![uri]),
        actions: DragActions::CopyOrMove,
        icon: ghost_icon(picture, look),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ds::{ImageSize, ImageSource};
    use shell_host::{DragAction, DragActions, DragOutcome, DragPayload, DragRefused};

    use super::{drag_end, file_drag};
    use crate::surfaces::screenshot_thumb::ghost::GhostLook;
    use crate::surfaces::screenshot_thumb::machine::DragEnd;
    use crate::surfaces::screenshot_thumb::picture::ThumbPicture;

    #[test]
    fn each_outcome_is_what_the_machine_hears() {
        let cases = [
            (
                DragOutcome::Dropped {
                    action: DragAction::Copy,
                },
                DragEnd::Copied,
            ),
            (
                DragOutcome::Dropped {
                    action: DragAction::Move,
                },
                DragEnd::Moved,
            ),
            (DragOutcome::Cancelled, DragEnd::Cancelled),
            (
                DragOutcome::Unsupported {
                    why: DragRefused::NoPress,
                },
                DragEnd::Refused,
            ),
        ];
        for (outcome, want) in cases {
            assert_eq!(drag_end(&outcome), want, "{outcome:?}");
        }
    }

    #[test]
    fn the_drag_carries_the_screenshot_as_a_file_uri() {
        let picture = |file: &str| ThumbPicture {
            file: PathBuf::from(file),
            image: ImageSource::png(&[]),
            size: ImageSize {
                width: 16,
                height: 10,
            },
        };
        let spec = file_drag(
            &picture("/tmp/shots/Screenshot 1.png"),
            GhostLook::default(),
        )
        .expect("an absolute path");
        assert_eq!(
            spec.payload,
            DragPayload::Uris(vec!["file:///tmp/shots/Screenshot%201.png".to_owned()])
        );
        assert_eq!(spec.actions, DragActions::CopyOrMove);
        assert!(file_drag(&picture("shots/a.png"), GhostLook::default()).is_none());
    }
}
