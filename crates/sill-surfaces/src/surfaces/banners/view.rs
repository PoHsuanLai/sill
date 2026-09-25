//! The banner column: one transparent Toast root as tall as the surface, `NotificationMetrics`
//! (the `notifications.*` geometry keys) written on a wrapper, holding quire's `BannerStack` of
//! the deck's listed cards, each quire's `NotificationCard` (through [`NoteCard`]) that pauses the
//! hold under the pointer and swipes away. It follows `use_notifications` and the
//! [`BannerLink`] (only the output the link names shows cards), maps its surface while any card
//! is drawn and unmaps it once the stack has played the last one out (`on_hidden`), and keeps
//! the input and blur regions on the cards (`elements.rs`).
//!
//! [`BannerLink`]: super::BannerLink

use dioxus::prelude::*;
use ds::{BannerKey, BannerStack, BlurState, Ds, Hover, Material, RootChrome, RootExtent, Swipe};
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_services::{NotificationId, Notifications};
use sill_settings::{BannerEntry, BannerPosition, NotificationsSettings};

use super::deck::{DeckIn, Entry, EntryKey, Showing};
use super::drive::{Column, use_column};
use super::elements::{CardShape, Regions, element_regions};
use super::layer::ROOM;
use super::machine::{BannerIn, BannerParams};
use super::placement::BannerLink;
use super::stack::stack;
use crate::hooks::{
    use_appearance, use_ds_scale, use_notifications, use_services, use_settings, use_workspaces,
};
use crate::surfaces::launcher::panel::active_look;
use crate::surfaces::notification_card::{
    CardPart, CardPlace, CardPress, NoteCard, notification_metrics, part_id, wall_now,
};

/// The column's stylesheet (lint-clean under `Profile::Strict`, `tests/banners_lint.rs`).
pub const BANNERS_CSS: &str = include_str!("../../style/banners.css");

/// The custom properties the column writes on itself, for the stylesheet lint.
pub const BANNER_VARS: [&str; 1] = ["--sill-banner-room"];

/// The column on `output`.
pub(super) fn banners(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(sill_services::OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
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
            style { {BANNERS_CSS} }
            ColumnView { output: output.id, settings: file.notifications.clone() }
        }
    }
}

/// The cards, inside the root.
#[component]
fn ColumnView(output: shell_host::OutputId, settings: NotificationsSettings) -> Element {
    let control = use_services().notifications_control;
    let column = use_column(BannerParams::new(&settings), control);
    let live = use_notifications();
    let link = use_context::<BannerLink>();
    let shows = use_watch(link.subscribe());
    let max = usize::from(settings.stack_max.0);
    use_follow_stack(column, live, shows, output, max);
    use_map_while_showing(column);
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
    let banners: Vec<ds::Banner> = deck
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
fn banner(column: Column, notes: &Notifications, entry: &Entry) -> Option<ds::Banner> {
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
                swipe: Swipe::Dismiss(swiped),
                on_hover: move |hover: Hover| column.hovered(key, hover),
                on_press: move |what: CardPress| column.pressed(key, card.clone(), &pressed, what),
            }
        }
    };
    Some(ds::Banner {
        key: BannerKey(key.0),
        card: element,
    })
}

/// The notification a card draws: the live one, else the copy the column kept.
fn note_of(
    notes: &Notifications,
    id: NotificationId,
    column: &Column,
) -> Option<sill_services::Notification> {
    notes
        .banner(id)
        .map(|b| b.note.clone())
        .or_else(|| column.kept(id))
}

/// quire's word for `notifications.banner_position`.
fn position(corner: BannerPosition) -> ds::BannerPosition {
    match corner {
        BannerPosition::TopRight => ds::BannerPosition::TopRight,
        BannerPosition::BottomRight => ds::BannerPosition::BottomRight,
    }
}

/// `notifications.banner_entry_direction` as the stack's entry edge (a swiped card still
/// leaves to the right).
fn entry(edge: BannerEntry) -> ds::BannerEntry {
    match edge {
        BannerEntry::FromRight => ds::BannerEntry::FromRight,
        BannerEntry::FromBelow => ds::BannerEntry::FromBelow,
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
    shows: ReadSignal<super::BannerOutput>,
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
                ds::BannerPosition::TopRight,
                "top",
            ),
            (
                BannerPosition::BottomRight,
                ds::BannerPosition::BottomRight,
                "bottom",
            ),
        ];
        for (corner, want, slug) in cases {
            assert_eq!(position(corner), want, "{corner:?}");
            assert_eq!(corner_slug(corner), slug, "{corner:?}");
        }
    }
}
