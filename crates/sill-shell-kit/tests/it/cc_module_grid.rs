//! ModuleGrid on a real Blitz document: its columns, gap and padding are
//! props (`control_center.grid_*`), the columns share the width, and a Full tile spans them all.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::control_center::module_grid::GridColumns;
use sill_shell_kit::control_center::module_tile_kind::TileSpan;
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 400,
    height: 300,
    scale_percent: 100,
};

/// Three columns, gap 6, padding 10: a tile per column, then a Full tile.
#[allow(non_snake_case)]
fn Three() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Popover,
            div { style: "width:320px",
                ModuleGrid { columns: GridColumns(3), gap: Px(6.0), padding: Px(10.0),
                    ModuleTile { glyph: Icon::Wifi, title: "Wi-Fi", value: Check::On, onclick: |_| {} }
                    ModuleTile { glyph: Icon::Bluetooth, title: "Bluetooth", value: Check::Off, onclick: |_| {} }
                    ModuleTile { glyph: Icon::Moon, title: "Focus", value: Check::Off, onclick: |_| {} }
                    ModuleTile { glyph: Icon::Play, title: "Now Playing", value: Check::Off, span: TileSpan::Full, onclick: |_| {} }
                }
            }
        }
    }
}

const TICK: Duration = Duration::from_millis(50);

fn rect(harness: &Harness, selector: &str) -> Rect {
    harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is not in the document:\n{}", harness.html()))
}

#[test]
fn three_columns_share_the_width_and_full_spans_them_all() {
    let mut harness = Harness::new(Three, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(TICK);
    // 320 less the padding 10 each side is 300; less two gaps of 6 is 288, 96 a column.
    let first = rect(&harness, ".ds-module-tile:nth-child(1)");
    let third = rect(&harness, ".ds-module-tile:nth-child(3)");
    assert_eq!((first.origin.x.0, first.origin.y.0), (10.0, 10.0));
    assert_eq!(first.size.width.0, 96.0);
    assert_eq!(third.origin.x.0, 10.0 + 2.0 * (96.0 + 6.0));
    assert_eq!(third.origin.y.0, first.origin.y.0, "three in one row");
    let full = rect(&harness, ".ds-module-tile:nth-child(4)");
    assert_eq!((full.origin.x.0, full.size.width.0), (10.0, 300.0));
}
