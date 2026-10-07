//! The Batteries widget's Medium row on a real Blitz layout (design/23 section 4.1,
//! M15): four places on the reference's fixed 80 pitch from 20 in, whatever the number of
//! batteries. One, two, three or four batteries take the first places and a bare track fills
//! each place left over, so two batteries never spread to the card's ends with the middle empty.

use dioxus::prelude::*;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::battery::device_glyph::Device;
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::battery::{BatteryCell, BatteryEntry, BatteryWidget};
use sill_shell_kit::widget::timeline::Timeline;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 240,
    scale_percent: 100,
};

/// How many batteries the row shows.
static COUNT: GlobalSignal<usize> = Signal::global(|| 4);

fn entry(count: usize) -> BatteryEntry {
    let devices = [
        Device::Laptop,
        Device::Phone,
        Device::Headphones,
        Device::Mouse,
    ];
    BatteryEntry::Devices(
        devices
            .into_iter()
            .take(count)
            .enumerate()
            .map(|(at, device)| BatteryCell {
                name: format!("device {at}"),
                device,
                state: BatteryState {
                    level: Fraction(500),
                    power: BatteryPower::Battery,
                    low_at: LowAt::default(),
                },
            })
            .collect(),
    )
}

#[allow(non_snake_case)]
fn Row() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: Motion::Reduced, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div { style: "padding:20px;{WidgetMetrics::default().style_attr()}",
                WidgetCard { widget: BatteryWidget, timeline: Timeline::now(entry(COUNT())), size: WidgetSize::Medium }
            }
        }
    }
}

/// The left edge of each ring in the row, from the card's own left edge.
fn ring_lefts(harness: &Harness) -> Vec<f32> {
    let card = harness.rect(".ds-widget").expect("the card");
    (1..=4)
        .map(|place| {
            let ring = harness
                .rect(&format!(
                    ".ds-batteries-cell:nth-child({place}) .ds-battery"
                ))
                .expect("a ring in every place");
            ring.origin.x.0 - card.origin.x.0
        })
        .collect()
}

#[test]
fn every_count_keeps_four_places_on_the_80_pitch_from_20_in() {
    let mut harness = Harness::new(Row, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    for count in [1, 2, 3, 4] {
        harness.within(|| *COUNT.write() = count);
        harness.advance(Duration::from_millis(50));
        assert_eq!(harness.count(".ds-batteries-cell"), 4, "{count}");
        assert_eq!(
            harness.count(".ds-batteries-cell[*|data-place=empty]"),
            4 - count,
            "{count}"
        );
        let lefts = ring_lefts(&harness);
        for (place, left) in lefts.iter().enumerate() {
            let want = 20.0 + 80.0 * place as f32;
            assert!(
                (left - want).abs() < 0.6,
                "{count} batteries: place {place} at {left}, want {want} ({lefts:?})"
            );
        }
        let tops: Vec<f32> = (1..=4)
            .filter_map(|place| {
                harness.rect(&format!(
                    ".ds-batteries-cell:nth-child({place}) .ds-battery"
                ))
            })
            .map(|ring| ring.origin.y.0)
            .collect();
        assert!(
            tops.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.6),
            "{count} batteries: an empty place sits level with a full one ({tops:?})"
        );
    }
}
