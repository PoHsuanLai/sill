//! The popup's size estimates (`control_center/geometry.rs`) against the control center as a
//! Harness lays it out on the fake services (`Clock::Virtual`, `PinnedAppearance`): each
//! module's estimate and each detail pane's covers its laid-out box, with only a few pixels to
//! spare (design/29 R9).

mod cc_support;

use cc_support::details::{center, ms, open};
use cc_support::fake;
use ds_native::Harness;
use sill_model::units::Stamp;
use sill_services::{Bluetooth, BtPower, Micros, Radio, Timeline, Wifi};
use sill_settings::{ControlCenterModule, ControlCenterSettings};
use sill_surfaces::surfaces::control_center::{CcPage, Detail, estimate_size, place};

/// The most an estimate may overshoot what is laid out.
const SPARE: f64 = 8.0;

/// A module's cell in the default grid (Keyboard Brightness draws nothing on the fakes).
struct Cell {
    module: ControlCenterModule,
    /// 1-based, as `:nth-child`.
    child: u16,
}

const CELLS: [Cell; 8] = [
    Cell {
        module: ControlCenterModule::Wifi,
        child: 1,
    },
    Cell {
        module: ControlCenterModule::Bluetooth,
        child: 2,
    },
    Cell {
        module: ControlCenterModule::Focus,
        child: 3,
    },
    Cell {
        module: ControlCenterModule::Display,
        child: 4,
    },
    Cell {
        module: ControlCenterModule::Sound,
        child: 5,
    },
    Cell {
        module: ControlCenterModule::NowPlaying,
        child: 6,
    },
    Cell {
        module: ControlCenterModule::Appearance,
        child: 7,
    },
    Cell {
        module: ControlCenterModule::Battery,
        child: 8,
    },
];

/// The estimate of `module` alone, without the panel's padding.
fn module_estimate(module: ControlCenterModule) -> f64 {
    let cc = ControlCenterSettings::default();
    let placed = place(&[module], cc.grid_columns.0);
    let pad = i32::from(cc.grid_padding_px.0);
    f64::from(estimate_size(&cc, CcPage::Modules, &placed, 0, 10_000).height - 2 * pad)
}

/// The estimate of a detail page with `rows` rows, without the panel's padding.
fn detail_estimate(detail: Detail, rows: usize) -> f64 {
    let cc = ControlCenterSettings::default();
    let pad = i32::from(cc.grid_padding_px.0);
    f64::from(estimate_size(&cc, CcPage::Detail(detail), &[], rows, 10_000).height - 2 * pad)
}

fn laid(harness: &Harness, selector: &str) -> f64 {
    let rect = harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is laid out"));
    f64::from(rect.size.height.0)
}

/// `what`'s estimate against its laid-out height, when it misses.
fn miss(what: &str, estimate: f64, laid: f64) -> Option<String> {
    let spare = estimate - laid;
    match (0.0..SPARE).contains(&spare) {
        true => None,
        false => Some(format!(
            "{what}: estimate {estimate} against the laid-out {laid}"
        )),
    }
}

fn covers(what: &str, estimate: f64, laid: f64) {
    if let Some(miss) = miss(what, estimate, laid) {
        panic!("{miss}");
    }
}

/// A track playing with a known position: Now Playing's tallest face (the position bar).
fn playing(feeds: &fake::Feeds) {
    let mut media = fake::media();
    for player in &mut media.players {
        player.timeline = Timeline::Known {
            position: Micros(65_000_000),
            length: Micros(545_000_000),
            at: Stamp(1),
        };
    }
    feeds.media.send_replace(media);
}

/// Every module of the default grid, with a track playing (Now Playing's tallest face).
#[test]
fn each_module_estimate_covers_its_cell() {
    let running = center(playing);
    let h = &running.harness;
    let misses: Vec<String> = CELLS
        .iter()
        .filter_map(|cell| {
            let selector = format!(".ds-module-grid > :nth-child({})", cell.child);
            miss(
                &format!("{:?}", cell.module),
                module_estimate(cell.module),
                laid(h, &selector),
            )
        })
        .collect();
    assert!(misses.is_empty(), "{misses:#?}");
}

/// Nothing playing is shorter: the estimate still covers it (the popup's spare is
/// transparent), so it is sized for the playing face.
#[test]
fn now_playing_is_estimated_for_a_track() {
    let running = center(|_| {});
    let laid = laid(&running.harness, ".ds-module-grid > :nth-child(6)");
    let estimate = module_estimate(ControlCenterModule::NowPlaying);
    assert!(estimate >= laid, "estimate {estimate} against {laid}");
}

/// The whole grid: the rows and the gaps between them.
#[test]
fn the_grid_estimate_covers_the_grid() {
    let running = center(playing);
    let cc = ControlCenterSettings::default();
    let modules: Vec<ControlCenterModule> = CELLS.iter().map(|cell| cell.module).collect();
    let placed = place(&modules, cc.grid_columns.0);
    let pad = i32::from(cc.grid_padding_px.0);
    let estimate =
        f64::from(estimate_size(&cc, CcPage::Modules, &placed, 0, 10_000).height - 2 * pad);
    covers(
        "the grid",
        estimate,
        laid(&running.harness, ".ds-module-grid"),
    );
}

/// What a detail pane lists.
#[derive(Debug, Clone, Copy)]
enum List {
    /// The fakes' networks or devices.
    Full,
    /// The radio or adapter off: the empty list's line.
    Empty,
}

fn feed(list: List) -> impl FnOnce(&fake::Feeds) {
    move |feeds: &fake::Feeds| match list {
        List::Full => {}
        List::Empty => {
            feeds.wifi.send_replace(Wifi {
                radio: Radio::Off,
                ..fake::wifi()
            });
            feeds.bluetooth.send_replace(Bluetooth {
                power: BtPower::Off,
                ..fake::bluetooth()
            });
        }
    }
}

/// Wi-Fi's and Bluetooth's panes, full and empty: the head, one row per network or device (or
/// the empty line), the foot.
#[test]
fn each_detail_estimate_covers_its_pane() {
    let cases = [
        (Detail::Wifi, "Wi-Fi details", List::Full),
        (Detail::Bluetooth, "Bluetooth details", List::Full),
        (Detail::Wifi, "Wi-Fi details", List::Empty),
        (Detail::Bluetooth, "Bluetooth details", List::Empty),
    ];
    let mut misses = Vec::new();
    for (detail, label, list) in cases {
        let mut running = center(feed(list));
        open(&mut running.harness, label);
        running.harness.advance(ms(100));
        let h = &running.harness;
        let rows = h.count(".sill-cc-rows > *");
        misses.extend(miss(
            &format!("{label} {list:?} ({rows} rows)"),
            detail_estimate(detail, rows),
            laid(h, ".sill-cc-detail"),
        ));
    }
    assert!(misses.is_empty(), "{misses:#?}");
}
