//! The banner column: one transparent Toast root as tall as the surface, `NotificationMetrics` (the
//! `notifications.*` geometry keys) written on a wrapper, holding quire's `BannerStack` of the
//! deck's listed cards, each quire's `NotificationCard` (through [`NoteCard`]) that pauses the hold
//! under the pointer and swipes away. It follows the notifications service's live banners and the
//! [`BannerLink`] (only the output the link names shows cards), maps its surface while any card is
//! drawn and unmaps it once the stack has played the last one out (`on_hidden`), and keeps the
//! input and blur regions on the cards (`elements.rs`).
//!
//! [`BannerLink`]: sill_ui_kit::links::BannerLink

use dioxus::prelude::*;
use ds::{Material, RootChrome, RootExtent};
use ds_shell::{BannerKey, BannerStack, Hover, NotificationSwipe};
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_model::notifications::{NotificationId, Notifications};
use sill_services::NotificationsService;
use sill_settings::{BannerEntry, BannerPosition, NotificationsSettings};

use super::deck::{DeckIn, Entry, EntryKey, Showing};
use super::deck_keys::KeysIn;
use super::elements::{CardShape, Regions, element_regions};
use super::io::{Column, use_column};
use super::keys::banner_key;
use super::layer::ROOM;
use super::stack::stack;
use super::step::{BannerIn, BannerParams};
use sill_ui_kit::hooks::{use_control, use_service_part, use_settings};
use sill_ui_kit::links::{BannerLink, KeyWhat, Transient};
use sill_ui_kit::notification_card::{
    CardPart, CardPlace, CardPress, NoteCard, notification_metrics, part_id, wall_now,
};
use sill_ui_kit::root::SurfaceRoot;

/// The column's stylesheet (lint-clean under `Profile::Strict`, `tests/banners_lint.rs`).
pub const BANNERS_CSS: &str = include_str!("style.css");

/// The custom properties the column writes on itself, for the stylesheet lint.
pub const BANNER_VARS: [&str; 1] = ["--sill-banner-room"];

/// The column on `output`.
pub(super) fn banners(output: OutputInfo) -> Element {
    let file = use_settings()();
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Toast,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            style { {BANNERS_CSS} }
            ColumnView { output: output.id, settings: file.notifications.clone() }
        }
    }
}

/// The cards, inside the root.
#[component]
fn ColumnView(output: shell_host::OutputId, settings: NotificationsSettings) -> Element {
    let control = use_control::<NotificationsService>();
    let link = use_context::<BannerLink>();
    let column = use_column(BannerParams::new(&settings), control, link.clone());
    let live = use_service_part::<NotificationsService, _>(|notes| notes.live.clone());
    let shows = use_watch(link.subscribe());
    let max = usize::from(settings.stack_max.0);
    use_follow_stack(column, live, shows, output, max);
    use_map_while_showing(column);
    use_keyboard_asks(column, &link);
    let deck = (column.deck)();
    let shapes: Vec<CardShape> = deck
        .drawn()
        .into_iter()
        .filter_map(|e| {
            Some(CardShape {
                note: e.card.newest()?,
                count: e.card.count(),
            })
        })
        .collect();
    use_regions(element_regions(&shapes));
    let notes = live();
    let banners: Vec<ds_shell::Banner> = deck
        .listed()
        .into_iter()
        .filter_map(|entry| banner(column, &notes, entry))
        .collect();
    let style = format!(
        "{}--sill-banner-room:{ROOM}px;",
        notification_metrics(&settings).style_attr()
    );
    rsx! {
        div {
            class: "sill-banners",
            "data-corner": corner_slug(settings.banner_position),
            style,
            onkeydown: move |event: KeyboardEvent| {
                if let Some(input) = banner_key(&event.key(), settings.banner_position) {
                    event.prevent_default();
                    event.stop_propagation();
                    column.keys(input);
                }
            },
            BannerStack {
                banners,
                position: position(settings.banner_position),
                entry: entry(settings.banner_entry_direction),
                on_hidden: move |key: BannerKey| column.hidden(EntryKey(key.0)),
            }
        }
    }
}

/// One listed card as quire's banner.
fn banner(column: Column, notes: &Notifications, entry: &Entry) -> Option<ds_shell::Banner> {
    let id = entry.card.newest()?;
    let note = note_of(notes, id, &column)?;
    let key = entry.key;
    let card = entry.card.clone();
    let pressed = note.clone();
    let swiped = EventHandler::new(move |()| column.input(DeckIn::Card(key, BannerIn::Swiped)));
    let element = rsx! {
        div { id: part_id(id, CardPart::Whole), class: "sill-banner",
            NoteCard {
                note,
                count: entry.card.count(),
                place: CardPlace::Banner,
                now: wall_now(),
                id: part_id(id, CardPart::Plate),
                swipe: NotificationSwipe::Dismiss(swiped),
                on_hover: move |hover: Hover| column.hovered(key, hover),
                on_press: move |what: CardPress| column.pressed(key, card.clone(), &pressed, what),
            }
        }
    };
    Some(ds_shell::Banner {
        key: BannerKey(key.0),
        card: element,
    })
}

/// The notification a card draws: the live one, else the copy the column kept.
fn note_of(
    notes: &Notifications,
    id: NotificationId,
    column: &Column,
) -> Option<sill_model::notifications::Notification> {
    notes
        .banner(id)
        .map(|b| b.note.clone())
        .or_else(|| column.kept(id))
}

/// quire's word for `notifications.banner_position`.
fn position(corner: BannerPosition) -> ds_shell::BannerPosition {
    match corner {
        BannerPosition::TopRight => ds_shell::BannerPosition::TopRight,
        BannerPosition::BottomRight => ds_shell::BannerPosition::BottomRight,
    }
}

/// `notifications.banner_entry_direction` as the stack's entry edge (a swiped card still
/// leaves to the right).
fn entry(edge: BannerEntry) -> ds_shell::BannerEntry {
    match edge {
        BannerEntry::FromRight => ds_shell::BannerEntry::FromRight,
        BannerEntry::FromBelow => ds_shell::BannerEntry::FromBelow,
    }
}

/// `data-corner` on the column: the stack stands at its top or its bottom.
fn corner_slug(corner: BannerPosition) -> &'static str {
    match corner {
        BannerPosition::TopRight => "top",
        BannerPosition::BottomRight => "bottom",
    }
}

/// Feed the deck the stack whenever the banners, the link or the cap change.
fn use_follow_stack(
    column: Column,
    live: ReadSignal<Notifications>,
    shows: ReadSignal<sill_ui_kit::links::BannerOutput>,
    output: shell_host::OutputId,
    max: usize,
) {
    use_effect(use_reactive!(|max| {
        let banners = live.read().banners.clone();
        let cards = match shows.read().shows_on(output) {
            true => stack(&banners, max),
            false => Vec::new(),
        };
        column.keep(&banners);
        column.input(DeckIn::Cards(cards));
    }));
}

/// Follow the link's keyboard events: `sill focus-latest` choosing the banners, and the
/// compositor taking the keyboard away.
fn use_keyboard_asks(column: Column, link: &BannerLink) {
    let keys = use_watch(link.subscribe_keys());
    let mut heard = use_hook(|| CopyValue::new(keys.peek().serial));
    use_effect(move || {
        let event = *keys.read();
        if *heard.peek() == event.serial {
            return;
        }
        heard.set(event.serial);
        match event.what {
            KeyWhat::Asked(Transient::Banner) => column.keys(KeysIn::Asked),
            KeyWhat::Lost(Transient::Banner) => column.keys(KeysIn::Lost),
            KeyWhat::Idle | KeyWhat::Asked(_) | KeyWhat::Lost(_) => {}
        }
    });
}

/// Keep the surface's input and blur regions on `regions`, sending them only when they change.
/// Nothing to do without a host (a test, an SSR render).
fn use_regions(regions: Regions) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(None::<Regions>));
    if last.peek().as_ref() == Some(&regions) {
        return;
    }
    if let Some(surface) = surface {
        surface.set_input_region(regions.input.clone());
        surface.set_blur(regions.blur.clone());
    }
    last.set(Some(regions));
}

/// Map the surface once a card is drawn; unmap it once none is (the stack has played the last
/// one out and said so).
fn use_map_while_showing(column: Column) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Showing::Nothing));
    use_effect(move || {
        let showing = column.deck.read().showing();
        if *applied.peek() == showing {
            return;
        }
        applied.set(showing);
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match showing {
                Showing::Cards => Visibility::Shown,
                Showing::Nothing => Visibility::Hidden,
            }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{corner_slug, position};
    use sill_settings::BannerPosition;

    #[test]
    fn the_corner_is_quires_position() {
        let cases = [
            (
                BannerPosition::TopRight,
                ds_shell::BannerPosition::TopRight,
                "top",
            ),
            (
                BannerPosition::BottomRight,
                ds_shell::BannerPosition::BottomRight,
                "bottom",
            ),
        ];
        for (corner, want, slug) in cases {
            assert_eq!(position(corner), want, "{corner:?}");
            assert_eq!(corner_slug(corner), slug, "{corner:?}");
        }
    }
}
