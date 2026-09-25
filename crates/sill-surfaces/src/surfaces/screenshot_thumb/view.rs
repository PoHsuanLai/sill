//! The thumbnail's root: one transparent Toast root as big as the surface, holding quire's
//! `BannerStack` at the bottom right with at most one row, the card ([`super::card`]). The
//! stack plays the card in from the right (`banner-in`, `--e-spring`) and out (`banner-out`,
//! `--e-exit`) and says when the exit has settled (`on_hidden`), so the motion is quire's; the
//! thumbnail card itself is quire gap Q181. It follows `use_shots` and the `BannerLink` through
//! [`super::follow`] and [`super::machine::step`].

use dioxus::prelude::*;
use ds::{Banner, BannerEntry, BannerKey, BannerPosition, BannerStack, BlurState, Ds, Material};
use ds::{RootChrome, RootExtent};
use shell_host::{OutputId, OutputInfo, use_watch};
use sill_services::{OutputName, Shot, ShotId};
use sill_settings::ShotThumbnail;

use super::SCREENSHOT_THUMB_CSS;
use super::card::ThumbCard;
use super::drive::{Thumb, use_thumb};
use super::follow::{Here, follow};
use super::layer::ROOM;
use super::machine::{ThumbIn, ThumbParams, ThumbState};
use crate::hooks::{
    use_appearance, use_ds_scale, use_services, use_settings, use_shots, use_workspaces,
};
use crate::surfaces::banners::BannerLink;
use crate::surfaces::launcher::panel::active_look;
use crate::surfaces::notification_card::swipe_metrics;

/// The thumbnail on `output`.
pub(super) fn thumb(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let params = ThumbParams {
        hold_ms: u32::from(file.screenshot.thumbnail_hold_ms.0),
        rest_ms: u32::from(file.notifications.hover_min_remaining_ms.0),
    };
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: Material::Toast,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {SCREENSHOT_THUMB_CSS} }
            Column {
                output: output.id,
                params,
                setting: file.screenshot.thumbnail,
                metrics: swipe_metrics(&file.notifications),
            }
        }
    }
}

/// The stack, inside the root.
#[component]
fn Column(
    output: OutputId,
    params: ThumbParams,
    setting: ShotThumbnail,
    metrics: ds::SwipeMetrics,
) -> Element {
    let thumb = use_thumb(params, use_services().shots_control);
    use_context_provider(|| thumb);
    let shots = use_shots();
    let link = use_context::<BannerLink>();
    let place = use_watch(link.subscribe());
    let mut kept = use_hook(|| CopyValue::new(None::<Shot>));
    use_effect(use_reactive!(|setting| {
        let here = match place.read().shows_on(output) {
            true => Here::This,
            false => Here::Elsewhere,
        };
        let shots = shots.read();
        if let Some(input) = follow(&shots, here, setting) {
            if let ThumbIn::Current(Some(_)) = input {
                kept.set(shots.current.clone());
            }
            thumb.input(input);
        }
    }));
    let banners: Vec<Banner> = match (*thumb.state.read(), kept.peek().clone()) {
        (ThumbState::Shown { id, .. }, Some(shot)) if shot.id == id => vec![Banner {
            key: key_of(id),
            card: rsx! { ThumbCard { shot, metrics } },
        }],
        _ => Vec::new(),
    };
    rsx! {
        div { class: "sill-thumb-column", style: "padding:{ROOM}px;",
            BannerStack {
                banners,
                position: BannerPosition::BottomRight,
                entry: BannerEntry::FromRight,
                on_hidden: move |key: BannerKey| settled(thumb, key),
            }
        }
    }
}

/// The stack has played `key` out: the machine hides if that is the shot it was leaving (a
/// replaced shot's row settles while the newer one shows, and is nothing).
fn settled(thumb: Thumb, key: BannerKey) {
    let state = *thumb.state.peek();
    if let ThumbState::Leaving { id, .. } = state
        && key_of(id) == key
    {
        thumb.input(ThumbIn::ExitSettled);
    }
}

/// A shot's row key (ids count up from 1; a daemon never takes four billion screenshots).
fn key_of(id: ShotId) -> BannerKey {
    BannerKey(u32::try_from(id.0).unwrap_or(u32::MAX))
}
