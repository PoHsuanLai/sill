//! What the thumbnail on one output is told about the service's shot: pure, table-tested.
//!
//! A shot whose preview is still being made is waited for (a few tens of ms), so the card never
//! slides in empty; the output the banners' link does not name, and `screenshot.thumbnail =
//! "hide"`, show nothing.

use sill_model::screenshots::{ShotPreview, Shots};
use sill_settings::ShotThumbnail;

use super::step::ThumbIn;

/// The machine's input for `shots`, or `None` while the current shot's preview is pending. `here`
/// is whether this output is the one that shows (the banners' `BannerLink`).
pub fn follow(shots: &Shots, here: bool, setting: ShotThumbnail) -> Option<ThumbIn> {
    match (here, setting, &shots.current) {
        (false, _, _) | (_, ShotThumbnail::Hide, _) | (_, _, None) => Some(ThumbIn::Current(None)),
        (true, ShotThumbnail::Show, Some(shot)) => match shot.preview {
            ShotPreview::Pending => None,
            ShotPreview::Ready(_) | ShotPreview::Original => Some(ThumbIn::Current(Some(shot.id))),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sill_model::screenshots::{Shot, ShotId, ShotPreview, Shots};
    use sill_settings::ShotThumbnail;

    use super::follow;
    use crate::screenshot_thumb::step::ThumbIn;

    fn with(preview: ShotPreview) -> Shots {
        Shots {
            current: Some(Shot {
                id: ShotId(3),
                path: PathBuf::from("/s/Screenshot_1.png"),
                preview,
            }),
            ..Shots::default()
        }
    }

    #[test]
    fn a_ready_shot_shows_here_only_and_a_pending_one_waits() {
        let ready = with(ShotPreview::Ready(PathBuf::from("/c/p.png")));
        let show = ShotThumbnail::Show;
        let cases = [
            (
                ready.clone(),
                true,
                show,
                Some(ThumbIn::Current(Some(ShotId(3)))),
            ),
            (
                with(ShotPreview::Original),
                true,
                show,
                Some(ThumbIn::Current(Some(ShotId(3)))),
            ),
            (with(ShotPreview::Pending), true, show, None),
            (ready.clone(), false, show, Some(ThumbIn::Current(None))),
            (
                ready,
                true,
                ShotThumbnail::Hide,
                Some(ThumbIn::Current(None)),
            ),
            (Shots::default(), true, show, Some(ThumbIn::Current(None))),
        ];
        for (shots, here, setting, want) in cases {
            assert_eq!(
                follow(&shots, here, setting),
                want,
                "{here:?} {setting:?} {shots:?}"
            );
        }
    }
}
