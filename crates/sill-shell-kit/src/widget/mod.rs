//! The widget interface (design/23-WIDGETS.md section 9): every widget, quire's and other apps',
//! plugs in through one trait, [`Widget`], and is drawn only through [`WidgetCard`], which owns
//! the card. A provider hands a [`Timeline`] of dated entries and a refresh policy;
//! [`use_widget`] picks the entry for now on the design system's clock; [`WidgetRegistry`]
//! lists what a host can place. An app in another process sends the same timeline as a
//! [`WireTimeline`] (section 9.5; the transport is not built yet).
//!
//! A trait and not an enum, unlike the closed vocabularies elsewhere: the set of widgets is
//! open (every app may bring its own), and each kind has its own entry type.

pub mod battery;
pub mod calendar;
pub(crate) mod card;
pub mod clock;
pub mod contract;
pub(crate) mod follow;
pub(crate) mod frame;
pub(crate) mod gallery;
pub(crate) mod gallery_book;
pub(crate) mod gallery_rows;
pub mod kind;
pub(crate) mod layout;
pub mod registry;
pub(crate) mod scope;
pub(crate) mod slot;
pub mod timeline;
pub(crate) mod use_widget;
pub(crate) mod wire;

pub use {
    calendar::LARGE_EVENTS,
    contract::fit,
    layout::{
        DesktopGrid, GridCell, LayoutError, Order, WidgetAt, WidgetEdit, WidgetLayout,
        WidgetPlacement, apply, cells,
    },
    registry::{TakenKind, UnsizedKind},
    timeline::REFRESH_FLOOR,
    wire::{WireRefresh, WireTimeline},
};
