//! The card a widget sits on. On the desktop: a quire `Surface` in `Material::Widget` (design/20
//! §1.14; its tint, edge, drop and 20 px corner are the material's), holding a bare
//! `ModulePanel` for the optional title row, and carrying [`super::widget_element_id`] as its id
//! so the layer's input and blur regions find it. In the notification center: quire's
//! `ModulePanel` on its tile plate (`--r-tile`, `--surface-2`, a hairline, the module's
//! padding), a rounded tile on the Popover as macOS's Notification Center widgets are. Both are
//! stand-ins until quire has a widget frame (FINDINGS "M10 freeze", Q182): the size classes on a
//! grid unit, the card's own padding and `bump` on a value change are the gap's. The one rule of
//! sill's own is the desktop card's inset, in a spacing token (`widget_frame.css`).

use dioxus::prelude::*;
use ds::{Icon, Material, ModulePanel, PanelPlate, Surface, Text, TileSpan};

use super::{WidgetHost, WidgetKind, WidgetSize, widget_element_id};

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_frame.css");

/// A frame's title row: its glyph and title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameHead {
    pub glyph: Icon,
    pub title: &'static str,
}

/// The title row `kind`'s frame draws, if any: the month heads itself (its month and arrows).
pub fn frame_head(kind: WidgetKind) -> Option<FrameHead> {
    let head = |glyph, title| Some(FrameHead { glyph, title });
    match kind {
        WidgetKind::Calendar => None,
        WidgetKind::UpNext => head(Icon::Clock, "Up Next"),
        WidgetKind::Battery => head(Icon::BatteryFull, "Batteries"),
        WidgetKind::NowPlaying => head(Icon::Play, "Now Playing"),
        WidgetKind::WorldClock => head(Icon::Clock, "World Clock"),
    }
}

/// The `data-host` word.
pub(crate) fn host_word(host: WidgetHost) -> &'static str {
    match host {
        WidgetHost::Center => "center",
        WidgetHost::Desktop => "desktop",
    }
}

/// The `data-size` word.
pub(crate) fn size_word(size: WidgetSize) -> &'static str {
    match size {
        WidgetSize::Small => "small",
        WidgetSize::Medium => "medium",
        WidgetSize::Large => "large",
    }
}

/// `children` in `kind`'s frame for `host`.
#[component]
pub fn WidgetFrame(
    kind: WidgetKind,
    host: WidgetHost,
    size: WidgetSize,
    children: Element,
) -> Element {
    let head = frame_head(kind);
    let glyph = head.map(|h| h.glyph);
    let title = head.map(|h| Text::from(h.title));
    let body = match host {
        WidgetHost::Center => rsx! {
            ModulePanel { glyph, title, span: TileSpan::Full, plate: PanelPlate::Tile, {children} }
        },
        WidgetHost::Desktop => rsx! {
            Surface { material: Material::Widget,
                div { class: "sill-widget-card",
                    ModulePanel { glyph, title, span: TileSpan::Full, plate: PanelPlate::Bare,
                        {children}
                    }
                }
            }
        },
    };
    rsx! {
        div {
            id: widget_element_id(kind),
            class: "sill-widget",
            "data-host": host_word(host),
            "data-size": size_word(size),
            {body}
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::Icon;

    use super::{FrameHead, frame_head};
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
            let want = want.map(|(glyph, title)| FrameHead { glyph, title });
            assert_eq!(frame_head(kind), want, "{kind:?}");
        }
    }
}
