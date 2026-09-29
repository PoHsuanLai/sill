//! What the thumbnail on one output is told about the service's shot: pure, table-tested.
//!
//! A shot whose preview is still being made is waited for (a few tens of ms), so the card never
//! slides in empty; the output the banners' link does not name, and `screenshot.thumbnail =
//! "hide"`, show nothing.

use sill_model::screenshots::{ShotPreview, Shots};
use sill_settings::ShotThumbnail;

use super::step::ThumbIn;

/// Whether this output is the one that shows (the banners' `BannerLink`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Here {
    This,
    Elsewhere,
}

/// The machine's input for `shots`, or `None` while the current shot's preview is pending.
pub fn follow(shots: &Shots, here: Here, setting: ShotThumbnail) -> Option<ThumbIn> {
    match (here, setting, &shots.current) {
        (Here::Elsewhere, _, _) | (_, ShotThumbnail::Hide, _) | (_, _, None) => {
            Some(ThumbIn::Current(None))
        }
        (Here::This, ShotThumbnail::Show, Some(shot)) => match shot.preview {
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

    use super::{Here, follow};
    use crate::surfaces::screenshot_thumb::step::ThumbIn;

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
                Here::This,
                show,
                Some(ThumbIn::Current(Some(ShotId(3)))),
            ),
            (
                with(ShotPreview::Original),
                Here::This,
                show,
                Some(ThumbIn::Current(Some(ShotId(3)))),
            ),
            (with(ShotPreview::Pending), Here::This, show, None),
            (
                ready.clone(),
                Here::Elsewhere,
                show,
                Some(ThumbIn::Current(None)),
            ),
            (
                ready,
                Here::This,
                ShotThumbnail::Hide,
                Some(ThumbIn::Current(None)),
            ),
            (
                Shots::default(),
                Here::This,
                show,
                Some(ThumbIn::Current(None)),
            ),
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
