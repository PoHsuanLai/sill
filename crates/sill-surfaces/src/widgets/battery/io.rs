//! The Batteries widget on quire's card, fed from the services.

use dioxus::prelude::*;
use ds::WidgetCard;
use sill_services::{BatteryService, BluetoothService};

use super::provider::{battery_entry, battery_rows, machine_name};
use crate::hooks::use_service;
use crate::widgets::{Place, WidgetKind, use_widget_wake, widget_element_id};

/// The batteries on quire's card: a new entry each time a level changes.
#[component]
pub fn BatteryCard(place: Place) -> Element {
    let battery = use_service::<BatteryService>()();
    let bluetooth = use_service::<BluetoothService>()();
    let machine = use_hook(|| {
        let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
        machine_name(&host)
    });
    let entry = battery_entry(&battery_rows(&battery, &bluetooth, &machine));
    rsx! {
        WidgetCard {
            widget: ds::BatteryWidget,
            timeline: ds::Timeline::now(entry),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::Battery),
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}
