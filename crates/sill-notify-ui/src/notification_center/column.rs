//! The column and its catcher. While the center is drawn: the transparent catcher under
//! everything (a press on it closes the center, as a click beside macOS's Notification Center
//! does) and quire's `Panel` at the right edge, `notifications.center_width_px` wide, in the
//! Popover material, inside a wrapper carrying `NotificationMetrics`. The panel slides in as it
//! mounts (keyed by the opening) and out when the link lets it go; its `on_hidden` unmaps the
//! surface.

use dioxus::prelude::*;
use ds::{Material, Panel, PanelEdge, PanelScrim, Px, Shown, WakeStamp};
use shell_host::OutputId;

use super::contents::Contents;
use super::io::{use_center_ctx, use_phase};
use super::labels::TITLE;
use sill_ui_kit::hooks::use_settings;
use sill_ui_kit::notification_card::notification_metrics;
use sill_ui_kit::panel::{Opening, Phase};
use sill_ui_kit::widgets::provide_widget_wake;

/// The column on `output`, while the center is drawn there.
#[component]
pub(super) fn Column(output: OutputId) -> Element {
    let phase = use_phase(output);
    let now = phase.now();
    let shown = match now {
        Phase::Up(_) => Shown::Visible,
        Phase::Hidden(_) | Phase::Leaving(_) => Shown::Hidden,
    };
    let Some(opening) = now.drawn() else {
        return rsx! {};
    };
    let on_hidden = EventHandler::new(move |()| phase.settled());
    rsx! {
        Opened { key: "{opening.0}", output, opening, shown, on_hidden }
    }
}

/// One opening's catcher and panel: its own handlers' state, so each opening starts closed, and
/// its own widget wake (`sill_ui_kit::widgets::WidgetWake`), so a reopened battery ring counts up
/// again (the widgets' own remount already would; the stamp also covers a host that keeps them
/// mounted across an opening).
#[component]
fn Opened(
    output: OutputId,
    opening: Opening,
    shown: Shown,
    on_hidden: EventHandler<()>,
) -> Element {
    let ctx = use_center_ctx();
    use_context_provider(|| ctx.clone());
    provide_widget_wake(WakeStamp(opening.0 as u32));
    let settings = use_settings()().notifications;
    let metrics = notification_metrics(&settings);
    let (catch, escape) = (ctx.clone(), ctx.clone());
    rsx! {
        div {
            class: "sill-nc-catcher",
            onpointerdown: move |_| catch.close(),
        }
        div { class: "sill-nc-scope", style: metrics.style_attr(),
            Panel {
                label: TITLE.to_owned(),
                shown,
                on_hidden,
                onclose: move |()| escape.close(),
                width: Px(metrics.center_width.0),
                edge: PanelEdge::Right,
                material: Material::Popover,
                scrim: PanelScrim::None,
                Contents { output }
            }
        }
    }
}
