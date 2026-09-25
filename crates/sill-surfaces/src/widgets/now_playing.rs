//! The Now Playing widget (M10 lane b): M5's Now Playing reused, not redrawn. The control
//! center's `NowPlayingModule` reads `use_now_playing`, sends `MediaCommand::Press` and draws on
//! a `ModulePanel` whose plate it takes from context; the widget mounts it as it is, with the
//! plate `Bare`, since the widget's frame already is its plate. The module's layout rules live
//! in the control center's stylesheet, which the widgets' hosts do not load, so
//! `widget_now_playing.css` carries the same few layout rules for its classes.

use dioxus::prelude::*;
use ds::PanelPlate;

use super::WidgetHost;
use super::frame::host_word;
use crate::surfaces::control_center::modules::NowPlayingModule;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_now_playing.css");

/// The current player, with its transport.
#[component]
pub fn NowPlayingWidget(host: WidgetHost) -> Element {
    use_context_provider(|| PanelPlate::Bare);
    rsx! {
        div { class: "sill-widget-now-playing", "data-host": host_word(host),
            NowPlayingModule {}
        }
    }
}
