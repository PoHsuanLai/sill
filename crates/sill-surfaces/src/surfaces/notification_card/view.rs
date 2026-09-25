//! The card's markup (lane b, FINDINGS "M6 banners"): quire parts only, laid out by
//! `style/notification_card.css`. The card is a quire `Surface` in the Toast material (a banner;
//! ink on paper under `notifications.banner_material = "inverse"`) or the Popover material (a
//! center row), which paints its own tint, corner and shadow; a group's two offset layers are
//! two more `Surface`s behind it; the close button is quire's `IconButton` on a round `Surface`.
//! Everything here waits on quire's own notification card (Q120).

use std::path::PathBuf;
use std::time::SystemTime;

use dioxus::prelude::*;
use ds::{
    Corner, Count, CountPlace, ExternalIcon, Glyph, Icon, IconButton, IconButtonVariant, IconPx,
    IconSize, IconSource, IconUrl, IconView, Material, Propagation, Px, Radius, Scheme, Surface,
};
use sill_services::apps::{icon_theme, resolve_icon};
use sill_services::{AppIcon, AppRef, Notification, UnixMs};
use sill_settings::BannerMaterial;

use super::actions::{CardActions, action_layout};
use super::body::body_text;
use super::icon::{CardArt, card_art, entry_for};
use super::metrics::CardMetrics;
use super::parts::{CardPart, part_id};
use super::{CardPlace, CardPress, Expansion, age, age_label, app_label};
use crate::hooks::{use_apps, use_settings};
use crate::surfaces::banners::CARD_RADIUS;

/// The card's stylesheet (lint-clean under `Profile::Strict`, `tests/banners_lint.rs`).
pub const CARD_CSS: &str = include_str!("../../style/notification_card.css");

/// Now, as the cards' ages count it.
pub fn wall_now() -> UnixMs {
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    UnixMs(i64::try_from(ms).unwrap_or(i64::MAX))
}

/// `note` as a card.
pub(super) fn card(
    note: Notification,
    count: usize,
    expansion: Expansion,
    place: CardPlace,
    now: UnixMs,
    on_press: EventHandler<CardPress>,
) -> Element {
    let settings = use_settings()().notifications;
    let apps = use_apps()();
    let metrics = CardMetrics::new(&settings);
    let scheme = ds::use_env().scheme;
    let entry = entry_for(&note.app, &apps);
    let art = use_art(&note.app, entry.map(|e| &e.icon), || {
        card_art(&note.app, entry, theme_icon)
    });
    let app = app_label(&note.app, entry.map(|e| e.name.as_str()));
    let body = body_text(&note.body, expansion, metrics.line_chars());
    let grouped = count > 1;
    let layout = action_layout(note.buttons().cloned().collect());
    let (material, theme) = material_of(place, settings.banner_material, scheme);
    let expanded = match expansion {
        Expansion::Expanded => "expanded",
        Expansion::Collapsed => "collapsed",
    };
    let corner = Corner::Px(Px(CARD_RADIUS as f32));
    rsx! {
        div {
            class: "sill-card-slot",
            "data-place": place_slug(place),
            "data-grouped": if grouped { "grouped" } else { "single" },
            style: metrics.style_attr(),
            if grouped {
                div { id: part_id(note.id, CardPart::Far), class: "sill-card-layer", "data-depth": "2",
                    Surface { material, theme, radius: corner, div { class: "sill-card-fill" } }
                }
                div { id: part_id(note.id, CardPart::Near), class: "sill-card-layer", "data-depth": "1",
                    Surface { material, theme, radius: corner, div { class: "sill-card-fill" } }
                }
            }
            div { id: part_id(note.id, CardPart::Face), class: "sill-card-face",
                Surface { material, theme, radius: corner,
                    div {
                        class: "sill-card",
                        "data-expansion": expanded,
                        onclick: move |_| on_press.call(CardPress::Body),
                        div { class: "sill-card-icon", {art_view(art, metrics.icon)} }
                        div { class: "sill-card-text",
                            div { class: "sill-card-head",
                                span { class: "sill-card-app", {app} }
                                if grouped {
                                    Count { value: u32::try_from(count).unwrap_or(u32::MAX), place: CountPlace::Item }
                                }
                                span { class: "sill-card-age", {age_label(age(note.arrived, now))} }
                            }
                            if !note.summary.is_empty() {
                                div { class: "sill-card-summary", {note.summary.clone()} }
                            }
                            if !body.is_empty() {
                                div { class: "sill-card-body", "data-expansion": expanded, {body} }
                            }
                            if expansion == Expansion::Expanded {
                                CardActions { layout, on_press }
                            }
                        }
                    }
                }
            }
            if expansion == Expansion::Expanded {
                div { id: part_id(note.id, CardPart::Close), class: "sill-card-close",
                    Surface { material, theme, radius: Corner::Token(Radius::Pill),
                        IconButton {
                            variant: IconButtonVariant::Strip,
                            icon: Icon::X,
                            label: "Close".to_owned(),
                            propagation: Propagation::Stop,
                            onclick: move |_| on_press.call(CardPress::Close),
                        }
                    }
                }
            }
        }
    }
}

/// The material a card paints, and the scheme it forces (ink on paper: the one opposite the
/// enclosing `scheme`).
fn material_of(
    place: CardPlace,
    look: BannerMaterial,
    scheme: Scheme,
) -> (Material, Option<Scheme>) {
    let inverse = match scheme {
        Scheme::Light => Scheme::Dark,
        Scheme::Dark => Scheme::Light,
    };
    match (place, look) {
        (CardPlace::Center, _) => (Material::Popover, None),
        (CardPlace::Banner, BannerMaterial::Toast) => (Material::Toast, None),
        (CardPlace::Banner, BannerMaterial::Inverse) => (Material::Toast, Some(inverse)),
    }
}

/// The card's art, looked up again only when the sender's app or its entry's icon changes (the
/// theme lookup reads the icon theme's setting).
fn use_art(app: &AppRef, entry: Option<&AppIcon>, look: impl FnOnce() -> CardArt) -> CardArt {
    let key = (app.clone(), entry.cloned());
    let mut cached = use_hook(|| CopyValue::new(None::<((AppRef, Option<AppIcon>), CardArt)>));
    if let Some((seen, art)) = cached.peek().as_ref()
        && *seen == key
    {
        return art.clone();
    }
    let art = look();
    cached.set(Some((key, art.clone())));
    art
}

fn place_slug(place: CardPlace) -> &'static str {
    match place {
        CardPlace::Banner => "banner",
        CardPlace::Center => "center",
    }
}

/// A theme icon name as a file, through the apps service's lookup.
fn theme_icon(name: &str) -> Option<PathBuf> {
    match resolve_icon(name, &icon_theme()) {
        AppIcon::File(path) => Some(path),
        AppIcon::None => None,
    }
}

/// The card's icon at `side` px.
fn art_view(art: CardArt, side: u16) -> Element {
    let size = IconSize::Px(IconPx(u8::try_from(side).unwrap_or(u8::MAX)));
    let external = |path: &PathBuf| {
        IconUrl::file(path)
            .ok()
            .map(|url| ExternalIcon { url, size })
    };
    let source = match &art {
        CardArt::Image(path) => external(path).map(IconSource::Image),
        CardArt::Symbolic(path) => external(path).map(IconSource::Symbolic),
        CardArt::Bell => None,
    };
    match source {
        Some(source) => rsx! {
            IconView { source, size }
        },
        None => rsx! {
            Glyph { icon: Icon::Bell, size }
        },
    }
}
