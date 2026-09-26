//! The launcher's small moments (M9, FINDINGS "M9 freeze" F655): a result row appearing and the
//! preview pane appearing, changing and waiting on a slow load. These are a hook point, not sill
//! CSS: quire's D0 small-details primitives (`use_detail`, moments `Appear`, `Pending`,
//! `Preview`) drive them once they land; until then [`use_pane_moment`] does nothing and the
//! pane changes at once (quire rule 4: motion only through quire's timers).

use sill_launcher::Preview;

/// A moment the pane or a row goes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneMoment {
    /// Nothing changed.
    Still,
    /// The pane opens (D0 `Appear`).
    Appear,
    /// It shows another row's preview (D0 `Preview`).
    Swap,
    /// Its content is loading past a frame (a text file's head, a PDF's page: D0 `Pending`).
    Pending,
    /// It closes.
    Leave,
}

/// The moment from `before` to `after` (`None` for a hidden pane). Lane d fills it: hidden to
/// shown is `Appear`, shown to hidden `Leave`, one preview to a different one `Swap`, the same
/// `Still`.
pub fn pane_moment(before: Option<&Preview>, after: Option<&Preview>) -> PaneMoment {
    let _ = (before, after);
    todo!("M9 fill: d-preview")
}

/// Hand `moment` to quire's small-details driver. A no-op until quire's D0 `use_detail` lands
/// (Q297); lane d, or the adoption after it, calls `ds::use_detail` here and nowhere else.
pub fn use_pane_moment(moment: PaneMoment) {
    let _ = moment;
}

/// A row's first appearance in this opening (the `rise` stagger of design/20 §1.3, D0
/// `Appear`), keyed by its place. A no-op until D0 lands.
pub fn use_row_moment(index: usize) {
    let _ = index;
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sill_launcher::Preview;

    use super::{PaneMoment, pane_moment};

    #[test]
    #[ignore = "M9 fill: d-preview"]
    fn moments_follow_the_pane() {
        let image = Preview::Image(PathBuf::from("/a.png"));
        let other = Preview::Image(PathBuf::from("/b.png"));
        let cases = [
            (None, None, PaneMoment::Still),
            (None, Some(&image), PaneMoment::Appear),
            (Some(&image), None, PaneMoment::Leave),
            (Some(&image), Some(&other), PaneMoment::Swap),
            (Some(&image), Some(&image), PaneMoment::Still),
        ];
        for (before, after, want) in cases {
            assert_eq!(pane_moment(before, after), want, "{before:?} -> {after:?}");
        }
    }
}
