//! A notification as quire's `NotificationCard`: sill finds the app's icon and name, writes the
//! age, turns the body's runs and the actions into quire's, and hears the card's presses as one
//! [`CardPress`]. The card's look (the plate, the clamp, hover, the close button, the group's
//! layers, the swipe) is quire's.

use std::path::PathBuf;
use std::time::SystemTime;

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::icon::url::IconUrl;
use sill_model::apps::AppIcon;
use sill_model::notifications::{AppRef, Href, Notification, UnixMs};
use sill_services::apps::{IconRoots, icon_theme, resolve_icon};
use sill_services::env::Dirs;
use sill_shell_kit::notifications::parts::{AppMark, Hover};
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;

use super::actions::card_actions;
use super::body::rich_body;
use super::icon::{card_art, entry_for};
use super::labels::{age, age_label, app_label, group_count};
use super::metrics::icon_size;
use super::use_tokens::shared_tokens;
use super::{CardPlace, CardPress};
use crate::app_icons::IconFile;
use crate::hooks::{use_service, use_services};
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
    pub common: Common,
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
    let side = icon_size(&shared_tokens());
    let app = use_app_mark(&note.app, IconSize::Px(side));
    let on_press = wiring.on_press;
    let material = material_of(place);
    let swipe_metrics = super::metrics::swipe_metrics();
    rsx! {
        Surface { material, theme: None, chrome: RootChrome::Transparent,
            NotificationCard {
                app,
                age: age_label(age(note.arrived, now)),
                summary: note.summary.clone(),
                body: rich_body(&note.body),
                count: group_count(count),
                actions: card_actions(&note, on_press),
                on_close: move |_| on_press.call(CardPress::Close),
                on_open: move |_| on_press.call(CardPress::Body),
                on_link: move |href: String| on_press.call(CardPress::Link(Href::new(&href))),
                on_hover: wiring.on_hover,
                material,
                icon_size: side,
                common: wiring.common,
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
    let dirs = use_services().dirs;
    let entry = entry_for(app, &apps);
    let art = use_art(app, entry.map(|e| &e.icon), || {
        card_art(app, entry, |name| theme_icon(name, &dirs))
    });
    AppMark {
        icon: icon_source(art.as_ref(), size),
        name: TextLine::from(app_label(app, entry.map(|e| e.name.as_str()))),
    }
}

/// The material a card paints: a banner the Toast, a center row the center's Popover.
fn material_of(place: CardPlace) -> Material {
    match place {
        CardPlace::Center => Material::Popover,
        CardPlace::Banner => Material::Toast,
    }
}

/// The card's art, looked up again only when the sender's app or its entry's icon changes (the
/// theme lookup reads the icon theme's setting).
fn use_art(
    app: &AppRef,
    entry: Option<&AppIcon>,
    look: impl FnOnce() -> Option<IconFile>,
) -> Option<IconFile> {
    let key = (app.clone(), entry.cloned());
    let mut cached =
        use_hook(|| CopyValue::new(None::<((AppRef, Option<AppIcon>), Option<IconFile>)>));
    if let Some((seen, art)) = cached.peek().as_ref()
        && *seen == key
    {
        return art.clone();
    }
    let art = look();
    cached.set(Some((key, art.clone())));
    art
}

/// A theme icon name as a file, through the apps service's lookup (the theme is the apps service's
/// setting under the config directory of `dirs`; a file is drawn only from the folders the host
/// serves, `Dirs::icon_file_roots`).
fn theme_icon(name: &str, dirs: &Dirs) -> Option<PathBuf> {
    let roots = IconRoots::new(dirs.icon_file_roots());
    match resolve_icon(name, &icon_theme(&dirs.config), &roots) {
        AppIcon::File(path) => Some(path),
        AppIcon::None => None,
    }
}

/// The card's icon at `size`: the file, else quire's bell.
fn icon_source(art: Option<&IconFile>, size: IconSize) -> IconSource {
    let external = |path: &PathBuf| {
        IconUrl::file(path)
            .ok()
            .map(|url| ExternalIcon { url, size })
    };
    let found = match art {
        Some(IconFile::Themed(path)) => external(path).map(IconSource::Image),
        Some(IconFile::Symbolic(path)) => external(path).map(IconSource::Symbolic),
        None => None,
    };
    found.unwrap_or(IconSource::Glyph(Icon::Bell))
}
