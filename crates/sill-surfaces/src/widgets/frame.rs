//! What sill tells quire's `WidgetFrame` (Q182) beyond the size and host: each kind's title row.
//! The card itself (the Widget material's plate on the desktop, a tile on the Popover in the
//! notification center), its corner, padding and footprint are quire's. `widget_frame.css`
//! keeps the one rule every widget shares: the quiet line of a widget with nothing to show.

use ds::{Icon, WidgetTitle};

use super::WidgetKind;

/// This file's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_frame.css");

/// The glyph and name of `kind`'s title row, if it has one: the month heads itself.
pub fn frame_head(kind: WidgetKind) -> Option<(Icon, &'static str)> {
    match kind {
        WidgetKind::Calendar => None,
        WidgetKind::UpNext => Some((Icon::Clock, "Up Next")),
        WidgetKind::Battery => Some((Icon::BatteryFull, "Batteries")),
        WidgetKind::NowPlaying => Some((Icon::Play, "Now Playing")),
        WidgetKind::WorldClock => Some((Icon::Clock, "World Clock")),
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
    fn every_kind_but_the_month_has_a_title_row() {
        let cases = [
            (WidgetKind::Calendar, None),
            (WidgetKind::UpNext, Some((Icon::Clock, "Up Next"))),
            (WidgetKind::Battery, Some((Icon::BatteryFull, "Batteries"))),
            (WidgetKind::NowPlaying, Some((Icon::Play, "Now Playing"))),
            (WidgetKind::WorldClock, Some((Icon::Clock, "World Clock"))),
        ];
        for (kind, want) in cases {
            assert_eq!(frame_head(kind), want, "{kind:?}");
        }
    }
}
