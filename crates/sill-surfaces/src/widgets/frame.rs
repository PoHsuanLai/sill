//! The card a widget sits on (M10 lane b fills it). On the desktop: a quire `Surface` in
//! `Material::Widget` (design/20 §1.14), `size`'s footprint on the grid, carrying
//! [`super::widget_element_id`] as its id so the layer's input and blur regions find it. In the
//! notification center: a rounded tile on the Popover, as macOS's Notification Center widgets
//! are. Both are stand-ins until quire has a widget frame (FINDINGS "M10 freeze", Q182): the
//! corner, the padding and the optional title row are the frame's, never this file's CSS.

use dioxus::prelude::*;

use super::{WidgetHost, WidgetKind, WidgetSize};

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_frame.css");

/// `children` in `kind`'s frame for `host`.
#[component]
pub fn WidgetFrame(
    kind: WidgetKind,
    host: WidgetHost,
    size: WidgetSize,
    children: Element,
) -> Element {
    let _ = (kind, host, size, children);
    rsx! {}
}
