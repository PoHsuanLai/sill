//! The sizing audit's specimen sheet (design/29-SIZING.md): the shell-scale controls as they
//! stand (toggle, slider, segmented, buttons, level capsule, module tile and panel, bar items),
//! light and dark, at 1x and 2x, plus the measured box of each part.
//!
//! `cargo run --release -p ds-blitz --example sizing_audit -- OUT_DIR`

use dioxus::prelude::*;
use ds::base::vocab::Muting;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::components::controls::button_model::{Answers, ImagePosition};
use ds::components::controls::segmented::Tracking;
use ds::components::controls::slider_model::SliderLook;
use ds::prelude::*;
use ds::style::icon::family::PlateFamily;
use ds::style::icon::plate_tint::PlateTint;
use ds::style::icon::retint::IconStyle;
use ds::style::icon::retint::Tint;
use ds::style::tokens::control_size::ControlSize;
use ds::style::tokens::status::StatusMetrics;
use ds_harness::{Harness, Viewport};
use sill_shell_kit::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

const PARTS: [&str; 16] = [
    ".ds-toggle",
    ".ds-toggle-knob",
    ".ds-slider",
    ".ds-slider-thumb",
    ".ds-slider-track",
    "#seg-small",
    "#seg-regular",
    "#btn-mini",
    "#btn-regular",
    ".ds-level-rail",
    ".ds-module-tile",
    ".ds-module-disc",
    ".ds-module-panel",
    ".ds-bar-item",
    ".ds-button",
    ".ds-plate",
];

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/sizing_audit".into()),
    );
    std::fs::create_dir_all(&dir).expect("out dir");
    for (theme, app) in [("light", Light as fn() -> Element), ("dark", Dark)] {
        for scale in [100u16, 200] {
            let viewport = Viewport {
                width: 720,
                height: 520,
                scale_percent: scale,
            };
            let mut harness = Harness::new(app, viewport);
            harness.advance(Duration::from_millis(400));
            if scale == 100 && theme == "light" {
                for part in PARTS {
                    match harness.rect(part) {
                        Some(r) => println!(
                            "{part:22} {:>6.1} x {:>5.1}",
                            r.size.width.0, r.size.height.0
                        ),
                        None => println!("{part:22} (absent)"),
                    }
                }
            }
            let image = harness.render().expect("render");
            let path = dir.join(format!("specimens-{theme}-{}x.png", scale / 100));
            image.save(&path).expect("save");
            println!("wrote {}", path.display());
        }
    }
}

#[allow(non_snake_case)]
fn Light() -> Element {
    rsx! { Sheet { theme: Theme::Light } }
}

#[allow(non_snake_case)]
fn Dark() -> Element {
    rsx! { Sheet { theme: Theme::Dark } }
}

#[component]
fn Sheet(theme: Theme) -> Element {
    let scheme = match theme {
        Theme::Dark => Scheme::Dark,
        _ => Scheme::Light,
    };
    let metrics = StatusMetrics {
        box_size: Px(22.0),
        glyph: Px(16.0),
    };
    let row = "display:flex;align-items:center;gap:12px;padding:10px 14px;";
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { theme, ..Appearance::default() }, material: Material::Window,
            div { style: "display:flex;flex-direction:column;gap:8px;padding:12px;",
                // The bar: 32 tall as sill's default, a title pill, three status items, the clock.
                Surface { material: Material::Bar, theme: Some(scheme),
                    div { style: "display:flex;align-items:center;gap:4px;height:32px;padding:0 8px;{metrics.style_attr()}",
                        MenuBarItem { shown: Shown::Visible, label: "Files", onclick: |_| {} }
                        MenuBarItem { label: "Edit", onclick: |_| {} }
                        span { style: "flex:1" }
                        MenuBarItem { image: ImagePosition::Only, icon: Icon::Wifi, label: "Wi-Fi", onclick: |_| {} }
                        MenuBarItem { image: ImagePosition::Only, icon: Icon::Volume2, label: "Volume", shown: Shown::Visible, onclick: |_| {} }
                        MenuBarItem { image: ImagePosition::Only, icon: Icon::Switches, label: "Control center", onclick: |_| {} }
                        MenuBarItem { label: "Sun 27 Sep 12:48", onclick: |_| {} }
                    }
                }
                Surface { material: Material::Popover, theme: Some(scheme),
                    div { style: "{row}",
                        Toggle { label: "On", value: Check::On, onchange: |_| {} }
                        Toggle { label: "Off", value: Check::Off, onchange: |_| {} }
                        div { style: "width:160px", Slider { label: "Level", value: Fraction(600), onchange: |_| {} } }
                        span { id: "btn-mini", Button { size: ControlSize::Mini, label: "Mini", onclick: |_| {} } }
                        span { id: "btn-regular", Button { answers: Answers::Return, label: "Regular", onclick: |_| {} } }
                    }
                    div { style: "{row}",
                        span { id: "seg-small", SegmentedControl { label: "Small", choices: Choice::pairs(vec![(0, "System".to_owned()), (1, "Light".to_owned()), (2, "Dark".to_owned())]), tracking: Tracking::SelectOne(0), size: ControlSize::Mini, onchange: |_| {} } }
                        span { id: "seg-regular", SegmentedControl { label: "Regular", choices: Choice::pairs(vec![(0, "System".to_owned()), (1, "Light".to_owned()), (2, "Dark".to_owned())]), tracking: Tracking::SelectOne(0), size: ControlSize::Regular, onchange: |_| {} } }
                    }
                }
                // The control center's modules at its 320 width, 12 padding, 8 gap.
                Surface { material: Material::Popover, theme: Some(scheme),
                    div { style: "width:320px;",
                        ModuleGrid {
                            ModuleTile { glyph: Icon::Wifi, title: "Wi-Fi", status: "Home", value: Check::On, onclick: |_| {}, on_detail: |_| {} }
                            ModuleTile { glyph: Icon::Bluetooth, title: "Bluetooth", status: "Off", value: Check::Off, onclick: |_| {}, on_detail: |_| {} }
                            ModulePanel { glyph: Icon::Sun, title: "Display", trailing: rsx! { "60%" },
                                Slider { label: "Display", value: Fraction(600), glyph: LevelGlyph::Volume(Muting::Audible), look: SliderLook::Capsule }
                            }
                        }
                    }
                }
                Surface { material: Material::Popover, theme: Some(scheme),
                    div { style: "{row}",
                        for (style , tint) in [(IconStyle::Colour, Tint::NEUTRAL), (IconStyle::Muted, Tint::NEUTRAL), (IconStyle::Monochrome, Tint { hue: 265.0, chroma: 0.05 })] {
                            for family in [PlateFamily::Neutral, PlateFamily::Blue, PlateFamily::Green] {
                                IconView { source: IconSource::Glyph(Icon::Window), size: IconSize::Tile48, plate: Some(family), plate_tint: PlateTint::of(style, tint) }
                            }
                        }
                    }
                }
            }
        }
    }
}
