//! BatteryRing: a battery's charge as a ring (design/30 section 2.10, design/23-WIDGETS.md
//! section 4.1), drawn flat and bright as the reference widget measures: a `ProgressIndicator`
//! ring (one arc geometry: a thick round-capped arc from twelve, clockwise, as far as the
//! level, over a full track) with the device's glyph centred in it, in `--battery-fill`
//! (`--battery-low` while the battery reads low); while charging, the ring is notched at twelve
//! and a bolt sits in the notch. One [`BatteryState`] says the level, the power and where low
//! begins, as it does for the bar's glyph and a row's accessory.
//!
//! Markup: `div.ds-battery-ring[data-tone][data-power]` holding `div.ds-battery` (the ring and
//! the bolt) and, under `Readout::Under`, the percentage in a `Label`.

use dioxus::prelude::*;
use ds::base::vocab::Fraction;
use ds::base::word::Word;
use ds::components::content::label::{Label, LabelRole, LabelStyle};
use ds::components::content::status::battery_state::{BatteryPower, BatteryState};
use ds::components::content::text_runs::TextLine;
use ds::components::controls::progress::model::{Progress, ProgressStyle, RingGap};
use ds::components::controls::progress::view::ProgressIndicator;
use ds::root::common::Common;

/// The charging bolt, in its own 10 x 16 box.
const BOLT: &str = "M7 0 0 9.6h4.6L3.2 16 10 6.4H5.4L7 0Z";

/// Whether the percentage is drawn under the ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum Readout {
    /// The ring alone: the caller draws the figure where its layout wants it.
    #[default]
    Alone,
    /// The percentage under the ring, centred.
    Under,
}

/// The percentage as text: `84%`.
pub fn percent_text(level: Fraction) -> String {
    format!("{}%", level.whole_percent())
}

/// A battery ring for `state`, named `label` for assistive technology. `children` (a device's
/// glyph, optional) sit in the ring's middle.
#[component]
pub fn BatteryRing(
    state: BatteryState,
    #[props(into)] label: TextLine,
    #[props(default)] readout: Readout,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    let class = common.class("ds-battery-ring");
    let data = common.data_attributes();
    let charging = state.power == BatteryPower::Charging;
    let (gap, tone) = match (charging, state.is_low()) {
        (true, _) => (RingGap::Notched, "ok"),
        (false, true) => (RingGap::Closed, "low"),
        (false, false) => (RingGap::Closed, "ok"),
    };
    let power = match state.power {
        BatteryPower::Battery => "battery",
        BatteryPower::Charging => "charging",
        BatteryPower::Held => "held",
    };
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-tone": tone,
            "data-power": power,
            onmounted: move |event| common.mounted(event),
            ..data,
            div { class: "ds-battery",
                ProgressIndicator {
                    style: ProgressStyle::Ring,
                    progress: Progress::Known(state.level),
                    gap,
                    centre: given(children),
                    common: Common { aria_label: Some(label.plain_text()), ..Common::default() },
                }
                if charging {
                    svg { class: "ds-battery-bolt", "data-ds-svg": "battery", view_box: "0 0 10 16", "aria-hidden": "true",
                        path { d: BOLT, fill: "currentColor" }
                    }
                }
            }
            if readout == Readout::Under {
                span { class: "ds-battery-figure",
                    Label {
                        text: percent_text(state.level),
                        role: LabelRole::Secondary,
                        style: LabelStyle::Footnote,
                    }
                }
            }
        }
    }
}

/// `children`, or `None` when the caller passed none: an omitted `children` is the shared
/// placeholder node.
pub(crate) fn given(children: Element) -> Option<Element> {
    match &children {
        Ok(node) if *node == VNode::placeholder() => None,
        _ => Some(children),
    }
}
