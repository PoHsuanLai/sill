//! The Batteries widget's feed, from the services.

use super::provider::{battery_entry, battery_rows, machine_name};
use crate::hooks::use_service;
use crate::widgets::{Feed, WidgetKind, WidgetProvider, WidgetSize};
use dioxus::prelude::*;
use sill_services::{BatteryService, BluetoothService};

/// The batteries: a new entry each time a level changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BatteryProvider;

impl WidgetProvider for BatteryProvider {
    type W = ds::BatteryWidget;
    const KIND: WidgetKind = WidgetKind::Battery;
    const DESKTOP_SIZE: WidgetSize = WidgetSize::Small;

    fn use_feed() -> Feed<ds::BatteryWidget> {
        let battery = use_service::<BatteryService>()();
        let bluetooth = use_service::<BluetoothService>()();
        let machine = use_hook(|| {
            let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
            machine_name(&host)
        });
        Feed {
            entry: battery_entry(&battery_rows(&battery, &bluetooth, &machine)),
            onintent: None,
        }
    }
}
