//! The banner column (lane b, FINDINGS "M6 banners"): a transparent Toast root as tall as the
//! surface holding the drawn cards in their corner, each a [`NotificationCard`] in a slot the
//! stylesheet moves (entering, following a drag, flying out). It follows `use_notifications` and
//! the [`BannerLink`] (only the output the link names shows cards), maps its surface while any
//! card is drawn and unmaps it once the last one's exit has played, and keeps the input and blur
//! regions on the cards (`measure.rs`).
//!
//! [`BannerLink`]: super::BannerLink

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material, RootChrome, RootExtent};
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_services::{NotificationId, Notifications};
use sill_settings::NotificationsSettings;

use super::deck::{DeckIn, Entry, Showing};
use super::drive::{Column, use_column};
use super::layer::ROOM;
use super::look::{card_look, corner_slug, entry_slug, in_corner};
use super::machine::BannerParams;
use super::measure::{RegionKey, Slots, use_card_regions};
use super::placement::BannerLink;
use super::stack::stack;
use crate::hooks::{
    use_appearance, use_ds_scale, use_notifications, use_services, use_settings, use_workspaces,
};
use crate::surfaces::launcher::panel::active_look;
use crate::surfaces::notification_card::{
    CARD_CSS, CardPlace, CardPress, MenuOpen, MenuSeat, NotificationCard, wall_now,
};

/// The column's stylesheet (lint-clean under `Profile::Strict`, `tests/banners_lint.rs`).
pub const BANNERS_CSS: &str = include_str!("../../style/banners.css");

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
            style { {CARD_CSS} }
            style { {BANNERS_CSS} }
            ColumnView { output: output.id, settings: file.notifications.clone() }
        }
    }
}

/// The cards, inside the root (its motion level is the timers').
#[component]
fn ColumnView(output: shell_host::OutputId, settings: NotificationsSettings) -> Element {
    let level = ds::use_env().resolved.motion;
    let control = use_services().notifications_control;
    let column = use_column(BannerParams::new(&settings), control, level);
    let live = use_notifications();
    let link = use_context::<BannerLink>();
    let shows = use_watch(link.subscribe());
    let menu = use_signal(|| MenuOpen::No);
    use_context_provider(|| MenuSeat(menu));
    let max = usize::from(settings.stack_max.0);
    use_follow_stack(column, live, shows, output, max);
    use_map_while_showing(column);
    let deck = (column.deck)();
    let drawn: Vec<Entry> = in_corner(
        deck.drawn().into_iter().cloned().collect(),
        settings.banner_position,
    );
    let key = RegionKey {
        cards: drawn
            .iter()
            .map(|e| (e.key, format!("{:?}{:?}{:?}", e.banner, e.expansion, e.card)))
            .collect(),
        menu: menu(),
    };
    let slots = use_card_regions(key);
    let notes = live();
    let from = entry_slug(settings.banner_entry_direction);
    let style = format!(
        "--sill-banner-room:{ROOM}px;--sill-banner-gap:{}px;--sill-banner-edge:{}px;",
        settings.stack_gap_px.0, settings.position_right_px.0
    );
    rsx! {
        div {
            class: "sill-banners",
            "data-corner": corner_slug(settings.banner_position),
            style,
            onmousemove: move |event| column.moved(event.client_coordinates().x),
            onmouseup: move |event| column.up(event.client_coordinates().x),
            for entry in drawn {
                Fragment { key: "{entry.key.0}", {slot(column, slots, &notes, entry, from, menu)} }
            }
        }
    }
}

/// One card's slot.
fn slot(
    column: Column,
    slots: Slots,
    notes: &Notifications,
    entry: Entry,
    from: &'static str,
    menu: Signal<MenuOpen>,
) -> Element {
    let Some(note) = entry.card.newest().and_then(|id| note_of(notes, id, &column)) else {
        return rsx! {};
    };
    let look = card_look(&entry);
    let key = entry.key;
    let card = entry.card.clone();
    let pressed = note.clone();
    rsx! {
        div {
            class: "sill-banner",
            "data-pose": look.pose,
            "data-transit": look.transit.slug(),
            "data-from": from,
            style: look.shift,
            onmounted: move |event| slots.mounted(key, event),
            onmouseenter: move |_| column.input(DeckIn::Card(key, super::BannerIn::HoverIn)),
            onmouseleave: move |_| {
                if menu() == MenuOpen::No {
                    column.input(DeckIn::Card(key, super::BannerIn::HoverOut));
                }
            },
            onmousedown: move |event| column.down(key, event.client_coordinates().x),
            NotificationCard {
                note,
                count: entry.card.count(),
                expansion: entry.expansion,
                place: CardPlace::Banner,
                now: wall_now(),
                on_press: move |what: CardPress| column.pressed(key, card.clone(), &pressed, what),
            }
        }
    }
}

/// The notification a card draws: the live one, else the copy the deck kept (a card sliding out
/// after the service dropped it).
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

/// Map the surface once a card is drawn; unmap it once none is (the last exit has played).
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
