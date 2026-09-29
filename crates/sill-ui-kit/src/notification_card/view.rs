//! A notification as quire's `NotificationCard`: sill finds the app's icon and name, writes the
//! age, turns the body's runs and the actions into quire's, and hears the card's presses as one
//! [`CardPress`]. The card's look (the plate, the clamp, hover, the close button, the group's
//! layers, the swipe) is quire's.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dioxus::prelude::*;
use ds::{
    AppMark, ExternalIcon, Hover, Icon, IconSize, IconSource, IconUrl, Material, NotificationCard,
    NotificationSwipe, RootChrome, Scheme, Surface, TextLine,
};
use sill_model::apps::AppIcon;
use sill_model::notifications::{AppRef, Href, Notification, UnixMs};
use sill_services::apps::{icon_theme, resolve_icon};
use sill_settings::{BannerMaterial, NotificationsSettings};

use super::actions::card_actions;
use super::body::rich_body;
use super::icon::{CardArt, card_art, entry_for};
use super::labels::{age, age_label, app_label, group_count};
use super::metrics::icon_size;
use super::{CardPlace, CardPress};
use crate::hooks::{use_section, use_service, use_services};
use sill_services::AppsService;

/// Now, as the cards' ages count it.
pub fn wall_now() -> UnixMs {
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    UnixMs(i64::try_from(ms).unwrap_or(i64::MAX))
}

/// What a card is told beyond its notification.
#[derive(Clone, PartialEq)]
pub(super) struct CardWiring {
    pub id: Option<String>,
    pub swipe: NotificationSwipe,
    pub on_hover: Option<EventHandler<Hover>>,
    pub on_press: EventHandler<CardPress>,
}

/// `note` as quire's card, standing for `count` notifications, drawn at `place`.
pub(super) fn card(
    note: Notification,
    count: usize,
    place: CardPlace,
    now: UnixMs,
    wiring: CardWiring,
) -> Element {
    let settings = use_section::<NotificationsSettings>()();
    let side = icon_size(&settings);
    let app = use_app_mark(&note.app, IconSize::Px(side));
    let on_press = wiring.on_press;
    let (material, theme) = material_of(place, settings.banner_material, ds::use_scope().scheme);
    let swipe_metrics = super::metrics::swipe_metrics(&settings);
    rsx! {
        Surface { material, theme, chrome: RootChrome::Transparent,
            NotificationCard {
                app,
                age: age_label(age(note.arrived, now)),
                summary: note.summary.clone(),
                body: rich_body(&note.body),
                count: group_count(count),
                actions: card_actions(&note, on_press),
                on_close: move |_| on_press.call(CardPress::Close),
                on_open: move |_| on_press.call(CardPress::Body),
                on_link: move |href: String| on_press.call(CardPress::Link(Href(href))),
                on_hover: wiring.on_hover,
                material,
                icon_size: side,
                id: wiring.id,
                swipe: wiring.swipe,
                swipe_metrics,
            }
        }
    }
}

/// Which app sent a notification, as quire names it: the icon (at `size`) and the name the card
/// and the center's group header show.
pub fn use_app_mark(app: &AppRef, size: IconSize) -> AppMark {
    let apps = use_service::<AppsService>()();
    let config = use_services().dirs.config;
    let entry = entry_for(app, &apps);
    let art = use_art(app, entry.map(|e| &e.icon), || {
        card_art(app, entry, |name| theme_icon(name, &config))
    });
    AppMark {
        icon: icon_source(&art, size),
        name: TextLine::from(app_label(app, entry.map(|e| e.name.as_str()))),
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

/// A theme icon name as a file, through the apps service's lookup (the theme is COSMIC's
/// setting under the config directory `config`).
fn theme_icon(name: &str, config: &Path) -> Option<PathBuf> {
    match resolve_icon(name, &icon_theme(config)) {
        AppIcon::File(path) => Some(path),
        AppIcon::None => None,
    }
}

/// The card's icon at `size`: the file, else quire's bell.
fn icon_source(art: &CardArt, size: IconSize) -> IconSource {
    let external = |path: &PathBuf| {
        IconUrl::file(path)
            .ok()
            .map(|url| ExternalIcon { url, size })
    };
    let found = match art {
        CardArt::Image(path) => external(path).map(IconSource::Image),
        CardArt::Symbolic(path) => external(path).map(IconSource::Symbolic),
        CardArt::Bell => None,
    };
    found.unwrap_or(IconSource::Glyph(Icon::Bell))
}
