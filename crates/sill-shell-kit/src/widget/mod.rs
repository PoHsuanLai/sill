//! The widget interface (design/23-WIDGETS.md section 9): the notification center's widgets plug
//! in through one trait, [`Widget`], and are drawn only through [`WidgetCard`], which owns the
//! card. A provider hands a [`Timeline`] of dated entries and a refresh policy; [`use_widget`]
//! picks the entry for now on the design system's clock.
//!
//! A trait and not an enum: each kind has its own entry type.

pub mod calendar;
pub(crate) mod card;
pub mod contract;
pub(crate) mod follow;
pub(crate) mod frame;
pub mod kind;
pub(crate) mod scope;
pub mod timeline;
pub(crate) mod use_widget;

pub use {calendar::LARGE_EVENTS, contract::fit, timeline::REFRESH_FLOOR};
