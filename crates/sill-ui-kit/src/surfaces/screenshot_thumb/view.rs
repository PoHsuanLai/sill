//! The thumbnail's root: one transparent Toast root as big as the surface, holding quire's
//! `BannerStack` at the bottom right with at most one row, quire's `ShotThumbnail`
//! ([`super::card`]). The card plays its own entrance and exit (`ShotIn`, `ShotOut`) and says when
//! the exit has settled; after a swipe the row is unlisted and the stack flies it out along the
//! swipe (`banner-out`) and says so instead. It follows `use_service::<ScreenshotsService>` and the
//! `BannerLink` through [`super::follow`] and [`super::step::step`].

use dioxus::prelude::*;
use ds::{Banner, BannerEntry, BannerKey, BannerPosition, BannerStack, Material};
use ds::{RootChrome, RootExtent, Shown};
use shell_host::{OutputId, OutputInfo, use_watch};
use sill_model::screenshots::ShotId;
use sill_services::ScreenshotsService;
use sill_settings::ShotThumbnail;

use super::card::ThumbCard;
use super::follow::{Here, follow};
use super::ghost::GhostLook;
use super::io::use_thumb;
use super::layer::ROOM;
use super::picture::{ThumbPicture, picture_of};
use super::step::{ThumbExit, ThumbIn, ThumbParams, ThumbState};
use crate::hooks::{use_appearance, use_control, use_service, use_settings};
use crate::links::BannerLink;
use crate::look::use_active_look;
use crate::notification_card::swipe_metrics;
use crate::root::SurfaceRoot;

/// The thumbnail's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const SCREENSHOT_THUMB_CSS: &str = include_str!("../../style/screenshot_thumb.css");

/// The thumbnail on `output`.
pub(super) fn screenshot_thumb(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let look = use_active_look(&output);
    let params = ThumbParams {
        hold_ms: u32::from(file.screenshot.thumbnail_hold_ms.0),
        rest_ms: u32::from(file.notifications.hover_min_remaining_ms.0),
    };
    let ghost = GhostLook::of(&environment, look.clone());
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Toast,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            style { {SCREENSHOT_THUMB_CSS} }
            Column {
                output: output.id,
                params,
                setting: file.screenshot.thumbnail,
                metrics: swipe_metrics(&file.notifications),
                ghost,
            }
        }
    }
}

/// What the stack lists for `state`, with the card's `shown`: the row while the shot shows or
/// plays its own exit, none once a swipe has handed the flight to the row.
fn row_of(state: ThumbState) -> Option<(ShotId, Shown)> {
    match state {
        ThumbState::Shown { id, .. } => Some((id, Shown::Visible)),
        ThumbState::Leaving {
            id,
            exit: ThumbExit::Card,
            ..
        } => Some((id, Shown::Hidden)),
        ThumbState::Leaving {
            exit: ThumbExit::Row,
            ..
        }
        | ThumbState::Hidden => None,
    }
}

/// The stack, inside the root.
#[component]
fn Column(
    output: OutputId,
    params: ThumbParams,
    setting: ShotThumbnail,
    metrics: ds::SwipeMetrics,
    ghost: GhostLook,
) -> Element {
    let thumb = use_thumb(params, use_control::<ScreenshotsService>());
    use_context_provider(|| thumb);
    let shots = use_service::<ScreenshotsService>();
    let link = use_context::<BannerLink>();
    let place = use_watch(link.subscribe());
    let mut kept = use_hook(|| CopyValue::new(None::<(ShotId, ThumbPicture)>));
    use_effect(use_reactive!(|setting| {
        let here = match place.read().shows_on(output) {
            true => Here::This,
            false => Here::Elsewhere,
        };
        let shots = shots.read();
        if let Some(input) = follow(&shots, here, setting) {
            if let (ThumbIn::Current(Some(id)), Some(shot)) = (input, &shots.current) {
                kept.set(picture_of(shot).map(|picture| (id, picture)));
            }
            thumb.input(input);
        }
    }));
    let banners: Vec<Banner> = match (row_of(*thumb.state.read()), kept.peek().clone()) {
        (Some((id, shown)), Some((kept_id, picture))) if kept_id == id => vec![Banner {
            key: key_of(id),
            card: rsx! {
                ThumbCard { id, picture, shown, metrics, look: ghost.clone() }
            },
        }],
        _ => Vec::new(),
    };
    rsx! {
        div { class: "sill-thumb-column", style: "padding:{ROOM}px;",
            BannerStack {
                banners,
                position: BannerPosition::BottomRight,
                entry: BannerEntry::FromRight,
                on_hidden: move |key: BannerKey| thumb.settled(id_of(key), ThumbExit::Row),
            }
        }
    }
}

/// A shot's row key (ids count up from 1; a daemon never takes four billion screenshots).
fn key_of(id: ShotId) -> BannerKey {
    BannerKey(u32::try_from(id.0).unwrap_or(u32::MAX))
}

/// The shot a row key names.
fn id_of(key: BannerKey) -> ShotId {
    ShotId(u64::from(key.0))
}

#[cfg(test)]
mod tests {
    use ds::Shown;
    use sill_model::screenshots::ShotId;

    use super::row_of;
    use crate::surfaces::screenshot_thumb::step::{HoldToken, Pointer, ThumbExit, ThumbState};

    #[test]
    fn the_row_stays_for_the_cards_own_exit_and_goes_for_a_swipe() {
        let id = ShotId(3);
        let token = HoldToken(1);
        let leaving = |exit| ThumbState::Leaving { id, token, exit };
        let cases = [
            (ThumbState::Hidden, None),
            (
                ThumbState::Shown {
                    id,
                    pointer: Pointer::Dragging,
                    token,
                },
                Some((id, Shown::Visible)),
            ),
            (leaving(ThumbExit::Card), Some((id, Shown::Hidden))),
            (leaving(ThumbExit::Row), None),
        ];
        for (state, want) in cases {
            assert_eq!(row_of(state), want, "{state:?}");
        }
    }
}
