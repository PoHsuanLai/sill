//! What sill tells quire's `WidgetFrame` (Q182) beyond the size and host: each kind's title row.
//! The card itself (the Widget material's plate on the desktop, a tile on the Popover in the
//! notification center), its corner, padding and footprint are quire's. `widget_frame.css`
//! keeps the one rule every widget shares: the quiet line of a widget with nothing to show.
//!
//! design/23-WIDGETS.md M25 (the fourth pass, "Flat, bright, measured"): no header on a
//! glanceable widget, the Batteries and Clock widgets' content fills the card. The gallery's
//! "Widget reference" page passes `WidgetFrame` no title for either, so `frame_head` gives
//! Battery and World Clock none.

use ds::{Icon, WidgetTitle};

use super::WidgetKind;

/// This file's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_frame.css");

/// The glyph and name of `kind`'s title row, if it has one: the month heads itself; Battery and
/// World Clock are glanceable widgets with no header (M25).
pub fn frame_head(kind: WidgetKind) -> Option<(Icon, &'static str)> {
    match kind {
        WidgetKind::Calendar | WidgetKind::Battery | WidgetKind::WorldClock => None,
        WidgetKind::UpNext => Some((Icon::Clock, "Up Next")),
        WidgetKind::NowPlaying => Some((Icon::Play, "Now Playing")),
    }
}

/// `kind`'s title row, as the frame takes it.
pub fn frame_title(kind: WidgetKind) -> Option<WidgetTitle> {
    frame_head(kind).map(|(glyph, text)| WidgetTitle::new(glyph, text))
}

#[cfg(test)]
mod tests {
    use ds::Icon;

    use super::frame_head;
    use crate::widgets::WidgetKind;

    #[test]
    fn only_the_lists_have_a_title_row() {
        let cases = [
            (WidgetKind::Calendar, None),
            (WidgetKind::UpNext, Some((Icon::Clock, "Up Next"))),
            (WidgetKind::Battery, None),
            (WidgetKind::NowPlaying, Some((Icon::Play, "Now Playing"))),
            (WidgetKind::WorldClock, None),
        ];
        for (kind, want) in cases {
            assert_eq!(frame_head(kind), want, "{kind:?}");
        }
    }
}
