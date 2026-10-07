//! NowPlayingTrack: the Now Playing module's art and titles (design/26-DETAILS.md 5.2.10).
//! A new track cross-fades in over `--t-quick` (the old art and titles fade out over the
//! new, which fade in); the same track restated plays nothing (R2); Reduced snaps (R7).

pub mod kind;
pub(crate) mod position;
pub(crate) mod track_position;

use dioxus::core::queue_effect;
use dioxus::prelude::*;
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::content::text_runs::{TextLine, text};
use ds::motion::detail::level::use_level;
use ds::motion::{
    anim::Anim,
    timer::{TimerPhase, use_motion_timer},
};
use ds::root::common::Common;
use ds::style::appearance::motion::MotionLevel;
use ds::style::icon::Icon;
use ds::style::icon::render::{Glyph, IconSize};

/// What the module shows of a track: compared whole, so a restated track is no moment.
#[derive(Debug, Clone, PartialEq)]
struct Face {
    art: Option<IconSource>,
    title: TextLine,
    by: Option<TextLine>,
}

/// The face shown, the one it replaced while the cross-fade plays, and which change it is.
#[derive(Debug, Clone, PartialEq)]
struct Faces {
    now: Face,
    before: Option<Face>,
    round: u32,
}

/// The track's art (48 px, or the Now Playing glyph on a plain well when there is none), its
/// `title` and `by` line. Hand it the track every render; a new one cross-fades in.
#[component]
pub fn NowPlayingTrack(
    #[props(default)] art: Option<IconSource>,
    #[props(into)] title: TextLine,
    #[props(default)] by: Option<TextLine>,
    #[props(default)] common: Common,
) -> Element {
    let face = Face { art, title, by };
    let timer = use_motion_timer(Anim::MorphFadeIn);
    let settled = use_hook(|| EventHandler::new(|()| {}));
    let level = use_level().now();
    let mut seen = use_hook(|| {
        CopyValue::new(Faces {
            now: face.clone(),
            before: None,
            round: 0,
        })
    });
    let was = seen.peek().clone();
    if was.now != face {
        let before = (level != MotionLevel::Reduced).then(|| was.now.clone());
        let fades = before.is_some();
        seen.set(Faces {
            now: face.clone(),
            before,
            round: was.round.wrapping_add(1),
        });
        queue_effect(move || {
            if fades {
                timer.start(settled);
            }
        });
    }
    let faces = seen.peek().clone();
    let fading = match (timer.phase(), &faces.before) {
        (TimerPhase::Running, Some(before)) => Some(before.clone()),
        (TimerPhase::Running | TimerPhase::Idle | TimerPhase::Settled, _) => None,
    };
    let alias = if faces.round.is_multiple_of(2) {
        "b"
    } else {
        "a"
    };
    let round = faces.round;
    let class = common.class("ds-track");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "aria-label": common.aria_label.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            span { class: "ds-track-art",
                if let Some(before) = &fading {
                    span { key: "art-out-{round}", class: "ds-track-layer a-morph-fade-out", "data-morph": "out", "data-pulse": alias,
                        {art_view(before.art.clone())}
                    }
                }
                span {
                    key: "art-in-{round}",
                    class: layer_class(fading.is_some()),
                    "data-morph": if fading.is_some() { "in" } else { "still" },
                    "data-pulse": fading.is_some().then_some(alias),
                    {art_view(faces.now.art.clone())}
                }
            }
            span { class: "ds-track-words",
                if let Some(before) = &fading {
                    span { key: "words-out-{round}", class: "ds-track-layer a-morph-fade-out", "data-morph": "out", "data-pulse": alias,
                        {words(before)}
                    }
                }
                span {
                    key: "words-in-{round}",
                    class: layer_class(fading.is_some()),
                    "data-morph": if fading.is_some() { "in" } else { "still" },
                    "data-pulse": fading.is_some().then_some(alias),
                    {words(&faces.now)}
                }
            }
        }
    }
}

/// The incoming layer's class: fading in while the cross-fade plays.
fn layer_class(fading: bool) -> &'static str {
    if fading {
        "ds-track-layer a-morph-fade-in"
    } else {
        "ds-track-layer"
    }
}

/// The art at 48, or the glyph on the well.
fn art_view(art: Option<IconSource>) -> Element {
    match art {
        Some(source) => rsx! {
            IconView { source, size: IconSize::Tile48 }
        },
        None => rsx! {
            span { class: "ds-track-none", Glyph { icon: Icon::Play, size: IconSize::Base } }
        },
    }
}

/// The title over the `by` line.
fn words(face: &Face) -> Element {
    rsx! {
        span { class: "ds-track-title ds-truncate", {text(&face.title)} }
        if let Some(by) = &face.by {
            span { class: "ds-track-by ds-truncate", {text(by)} }
        }
    }
}
