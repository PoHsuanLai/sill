//! The Now Playing widget (M10 lane b): M5's Now Playing reused, not redrawn. The control
//! center's `NowPlayingModule` reads `use_now_playing`, sends `MediaCommand::Press` and draws on
//! a `ModulePanel` with the control center's plate context; lane b either mounts it with that
//! context provided or lifts its body into a component both use (the control center's module
//! then becomes a thin wrapper, a change inside `control_center/modules/now_playing.rs` that
//! keeps its frozen name and props).

use dioxus::prelude::*;

use super::WidgetHost;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_now_playing.css");

/// The current player, with its transport.
#[component]
pub fn NowPlayingWidget(host: WidgetHost) -> Element {
    let _ = host;
    rsx! {}
}
