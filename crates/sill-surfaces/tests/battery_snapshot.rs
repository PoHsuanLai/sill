//! A picture of the Battery widget (design/23-WIDGETS.md section 4.1, the fourth pass "Flat,
//! bright, measured"), rendered headless (quire's `ds_native::snapshot`) to `target/`, for a
//! side-by-side with the reference's own gallery ("Widget reference" page, `ds-gallery`'s
//! `widget_reference.rs`): a Small frame with one device (the ring at the top left, the
//! percentage as the hero figure), a Small frame with several (the 2 x 2 grid, empty places
//! padded in), and a Medium frame (a row of rings, the percentage under each, one low and one
//! charging). The private session bus this widget runs against for real
//! (`dev/accept-widgets.sh`) has no batteries at all, so this is the only picture of it short of
//! a live capture. Review material only, never a gate (as `ds_native::snapshot`'s own doc
//! comment says): the rasteriser changes with Blitz revisions.

#[path = "support/fake.rs"]
mod fake;

use std::cell::RefCell;
use std::path::PathBuf;

use dioxus::prelude::*;
use ds::{Ds, Material, RootExtent};
use ds_native::{Viewport, snapshot};
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{
    Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
};
use sill_model::units::{Percent, Presence};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::services::Services;
use sill_surfaces::widgets::{Widget, WidgetHost, WidgetKind, widgets_css};

/// What the snapshot's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    host: WidgetHost,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    let material = match world.host {
        WidgetHost::Tile => Material::Popover,
        WidgetHost::Desktop => Material::Widget,
    };
    rsx! {
        Ds { appearance: ds::Appearance::default(), material, extent: RootExtent::Viewport,
            style { {widgets_css()} }
            div { style: "padding:24px;",
                Widget { kind: WidgetKind::Battery, host: world.host }
            }
        }
    }
}

const VIEWPORT: Viewport = Viewport {
    width: 500,
    height: 300,
    scale_percent: 200,
};

/// A connected device at `level`, its own kind and address.
fn device(address: &str, name: &str, kind: BtKind, level: u16) -> BtDevice {
    BtDevice {
        address: BtAddress(address.to_owned()),
        name: name.to_owned(),
        kind,
        connection: BtConnection::Connected,
        pairing: BtPairing::Paired,
        battery: BtBattery::Level(Percent(level)),
    }
}

fn battery_at(level: u16, charge: Charge) -> Battery {
    Battery {
        presence: Presence::Present,
        level: Percent(level),
        charge,
        ..fake::battery()
    }
}

/// This machine alone at 93 %, discharging (the reference's `R-bs`): a Small frame is the
/// solo layout.
fn solo_fixtures() -> (Battery, Bluetooth) {
    (battery_at(93, Charge::Discharging), Bluetooth::default())
}

/// This machine and one headset (the reference's `R-bg`, minus the two empty places our own
/// widget pads in): a Small frame is the 2 x 2 grid.
fn grid_fixtures() -> (Battery, Bluetooth) {
    let bluetooth = Bluetooth {
        devices: vec![device("AA:01", "Headphones", BtKind::Headphones, 80)],
        ..fake::bluetooth()
    };
    (battery_at(93, Charge::Discharging), bluetooth)
}

/// Four batteries as the reference's own medium row (`R-bm`, `R-bmm`): this machine charging at
/// 99 % (the bolt), a mouse low at 13 % (red), headphones at 80 %, a keyboard at 96 %.
fn row_fixtures() -> (Battery, Bluetooth) {
    let bluetooth = Bluetooth {
        devices: vec![
            device("AA:01", "Headphones", BtKind::Headphones, 80),
            device("AA:02", "Keyboard", BtKind::Keyboard, 96),
            device("AA:03", "Mouse", BtKind::Mouse, 13),
        ],
        ..fake::bluetooth()
    };
    (battery_at(99, Charge::Charging), bluetooth)
}

/// Snapshots `host` fed `battery` and `bluetooth`, to `target/battery-snapshots/<name>.png`.
fn shoot(name: &str, host: WidgetHost, battery: Battery, bluetooth: Bluetooth) -> PathBuf {
    let (services, feeds) = fake::services();
    feeds.battery.send_replace(battery);
    feeds.bluetooth.send_replace(bluetooth);
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(ShellFile::default()),
            host,
        });
    });
    let image = snapshot(app, VIEWPORT).unwrap_or_else(|e| panic!("{name}: {e}"));
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/battery-snapshots");
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{dir:?}: {e}"));
    let path = dir.join(format!("{name}.png"));
    image
        .save(&path)
        .unwrap_or_else(|e| panic!("{path:?}: {e}"));
    path
}

#[test]
fn the_battery_widget_snapshots_every_layout() {
    let (battery, bluetooth) = solo_fixtures();
    let solo = shoot(
        "battery-solo-small",
        WidgetHost::Desktop,
        battery,
        bluetooth,
    );
    assert!(solo.is_file());

    let (battery, bluetooth) = grid_fixtures();
    let grid = shoot(
        "battery-grid-small",
        WidgetHost::Desktop,
        battery,
        bluetooth,
    );
    assert!(grid.is_file());

    let (battery, bluetooth) = row_fixtures();
    let row = shoot("battery-row-medium", WidgetHost::Tile, battery, bluetooth);
    assert!(row.is_file());

    eprintln!("battery snapshots: {solo:?}, {grid:?}, {row:?}");
}
