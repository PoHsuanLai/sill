//! `Osd`: the on-screen display's card and its fade (design/20 section 1.7).
//! One card, a title line over a [`LevelIndicator`], in the Osd material with the
//! Space gradient at its tint, styled as a control-center module (`--r-tile`, the grid's padding,
//! design/13 section 13.3.7). It fades in with `Anim::PaletteFade` and fades out with `Anim::OsdOut`, and calls
//! `on_hidden` when the exit has settled, so the host can unmap the surface. The hold is the caller's: it knows `osd.hold_ms`.
//!
//! **Where it goes.** Put it directly inside a transparent Osd root:
//! `Ds { material: Material::Osd, chrome: Some(RootChrome::Transparent), stack, tint_alpha, look,
//! .. }`. The root paints nothing and gives the card every token (the motion tokens its fade
//! reads, the frame's `--f-*`, the material's `--m-*`, the stack and tint alpha); the card paints
//! what a tinted root would on its own box. One root, not a painted root nested in a transparent
//! one. The card's margin from its edge is `--osd-margin` (`osd.margin_px`,
//! written by [`crate::tokens::osd::OsdMetrics::style_attr`] on any element around it); a host sizes its
//! surface to the card, its margins and the material's shadow, anchored to that edge.

use dioxus::prelude::*;
use ds::base::vocab::Fraction;
use ds::base::vocab::Shown;
use ds::base::word::Word;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::components::controls::level_indicator::{LevelIndicator, LevelStyle};
use ds::motion::anim::Anim;
use ds::motion::presence::Exit;
use ds::motion::presence::spec::PresenceSpec;
use ds::motion::presence::use_presence::{Presented, use_presence};
use ds::root::common::Common;

/// The level an OSD shows: the value and the glyph that follows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OsdLevel {
    /// The level, 0 to 1000.
    pub value: Fraction,
    /// The speaker (heard or muted) or the sun.
    pub glyph: LevelGlyph,
}

/// Where the card sits (`osd.position`, design/22 section 3.16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum OsdPosition {
    /// Top right, under the bar's reserve, as current macOS shows its volume and brightness
    /// panel: the card drops in from above and lifts away (user, 2026-09-25).
    #[default]
    TopRight,
    /// Bottom centre, above the dock's reserve: it rises in and drops away.
    BottomCentre,
}

/// The on-screen display. `shown` is the caller's, and `on_hidden` runs once the card has faded
/// out after a hide; a show while it fades takes the hide back (it is present again at once and
/// `on_hidden` does not run for that hide).
#[component]
pub fn Osd(
    shown: Shown,
    #[props(default)] level: Option<OsdLevel>,
    #[props(default)] label: Option<String>,
    #[props(default)] on_hidden: EventHandler<()>,
    #[props(default)] position: OsdPosition,
    #[props(default)] style: LevelStyle,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    let spec = PresenceSpec {
        enter: Anim::PaletteFade,
        exit: Exit::OsdOut,
    };
    let Presented {
        presence: now,
        alias,
    } = use_presence(shown, spec, Some(on_hidden));
    let level_label = label.clone().unwrap_or_else(|| "Level".to_owned());
    let named = common.aria_label.clone().or_else(|| label.clone());
    let class = common.class("ds-osd");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            role: "status",
            "aria-label": named,
            onmounted: move |event| common.mounted(event),
            "data-position": position.slug(),
            "data-shown": now.shown().slug(),
            "data-presence": now.drawn_slug(),
            "data-pulse": alias.slug(),
            ..data,
            div { class: "ds-frame",
                div { class: "ds-grain" }
            }
            if let Some(title) = label {
                div { class: "ds-osd-title", "{title}" }
            }
            if let Some(OsdLevel { value, glyph }) = level {
                LevelIndicator {
                    label: level_label,
                    value,
                    glyph,
                    style,
                }
            }
            {children}
        }
    }
}
