//! The widget parts as markup (design/04-COMPONENTS.md "Widgets"):
//! the frame in each size on the desktop and as a tile, and the battery at three levels and charging. Each golden is
//! `tests/snapshots/widgets/<name>.html`; each lints clean and every `ds-` class in it is
//! styled by the stylesheet.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it widgets_ssr` rewrites the goldens.

use crate::support::specimens::Specimen;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::icon::render::Glyph;
use sill_shell_kit::battery::ring::percent_text;
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::kind::{CardTint, WidgetTitle};

#[derive(Props, Clone)]
struct HostProps {
    make: fn() -> Element,
}

impl PartialEq for HostProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn host(props: HostProps) -> Element {
    (props.make)()
}

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// `body` on the desktop layer: a transparent Widget root with the grid unit written, as sill's
/// desktop surface draws it.
fn desktop(theme: Theme, body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent), stylesheet: Inject::Host,
            div { style: WidgetMetrics::default().style_attr(), {body} }
        }
    }
}

/// `body` on the desktop under Reduced motion: a battery ring draws its level on its first
/// frame instead of sweeping to it, so its golden is the ring at rest.
fn settled(theme: Theme, body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme, motion: Motion::Reduced, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent), stylesheet: Inject::Host,
            div { style: WidgetMetrics::default().style_attr(), {body} }
        }
    }
}

/// `body` in the notification center: a Popover root.
fn center(body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover, stylesheet: Inject::Host,
            div { style: WidgetMetrics::default().style_attr(), {body} }
        }
    }
}

fn batteries() -> Option<WidgetTitle> {
    Some(WidgetTitle::new(Icon::BatteryFull, "Batteries"))
}

type Case = (&'static str, fn() -> Element);

/// A draining battery at `level` permille.
fn at(level: u16) -> BatteryState {
    BatteryState {
        level: Fraction(level),
        power: BatteryPower::Battery,
        low_at: LowAt::default(),
    }
}

/// A charging battery at `level` permille.
fn charging(level: u16) -> BatteryState {
    BatteryState {
        power: BatteryPower::Charging,
        ..at(level)
    }
}

const CASES: &[Case] = &[
    ("frame-small-desktop", || {
        desktop(
            Theme::Light,
            rsx! { WidgetFrame { size: WidgetSize::Small, "84%" } },
        )
    }),
    ("frame-medium-desktop-titled", || {
        desktop(
            Theme::Light,
            rsx! { WidgetFrame { size: WidgetSize::Medium, title: batteries(), common: Common { id: Some("widget-battery".to_string()), ..Common::default() }, "84%" } },
        )
    }),
    ("frame-large-desktop-dark", || {
        desktop(
            Theme::Dark,
            rsx! { WidgetFrame { size: WidgetSize::Large, "Up next" } },
        )
    }),
    ("frame-medium-desktop-space", || {
        desktop(
            Theme::Light,
            rsx! { WidgetFrame { size: WidgetSize::Medium, tint: CardTint::Space, "84%" } },
        )
    }),
    ("frame-medium-desktop-material", || {
        desktop(
            Theme::Light,
            rsx! { WidgetFrame { size: WidgetSize::Medium, tint: CardTint::Material, "84%" } },
        )
    }),
    ("frame-small-tile", || {
        center(rsx! { WidgetFrame { size: WidgetSize::Small, host: WidgetHost::Tile, "84%" } })
    }),
    ("frame-medium-tile-titled", || {
        center(
            rsx! { WidgetFrame { size: WidgetSize::Medium, host: WidgetHost::Tile, title: batteries(), "84%" } },
        )
    }),
    ("frame-large-tile", || {
        center(rsx! { WidgetFrame { size: WidgetSize::Large, host: WidgetHost::Tile, "Up next" } })
    }),
    ("battery-8", || {
        settled(
            Theme::Light,
            rsx! { BatteryRing { state: at(80), label: "Mouse" } },
        )
    }),
    ("battery-45", || {
        settled(
            Theme::Light,
            rsx! { BatteryRing { state: at(450), label: "Headphones", Glyph { icon: Icon::Headphones, size: IconSize::Base } } },
        )
    }),
    ("battery-100", || {
        settled(
            Theme::Dark,
            rsx! { BatteryRing { state: at(1000), label: "Keyboard" } },
        )
    }),
    ("battery-charging-15", || {
        settled(
            Theme::Light,
            rsx! { BatteryRing { state: charging(150), label: "This computer" } },
        )
    }),
];

fn html(name: &str) -> String {
    let (_, make) = CASES
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("no case {name}"));
    render(*make)
}

pub(crate) fn specimens() -> Vec<Specimen> {
    CASES
        .iter()
        .map(|(name, make)| Specimen::of(&format!("widgets/{name}.html"), render(*make)))
        .collect()
}

/// The desktop card is in a Widget scope and the tile in none of its own; both say their size
/// and host, and the title row is drawn only when asked for. Every desktop card is tinted by
/// the Space unless a host asks for the bare material (design/23 section 6, settled
/// 2026-09-27); a tile never is.
#[test]
fn a_frame_says_its_size_and_host_and_only_the_desktop_has_a_material() {
    let desktop = html("frame-medium-desktop-titled");
    assert!(desktop.contains("data-size=\"medium\""), "{desktop}");
    assert!(desktop.contains("data-host=\"desktop\""), "{desktop}");
    assert!(desktop.contains("id=\"widget-battery\""), "{desktop}");
    assert!(desktop.contains("ds-widget-title"), "{desktop}");
    assert_eq!(
        desktop.matches("data-material=\"widget\"").count(),
        2,
        "root and card scope"
    );
    let tile = html("frame-medium-tile-titled");
    assert!(tile.contains("data-host=\"tile\""), "{tile}");
    assert!(!tile.contains("data-material=\"widget\""), "{tile}");
    assert_eq!(
        tile.matches("data-material=").count(),
        1,
        "the popover root only"
    );
    assert!(!html("frame-small-tile").contains("ds-widget-title"));
    assert!(
        desktop.contains("data-tint=\"space\""),
        "every desktop card takes the Space's tint by default: {desktop}"
    );
    let material = html("frame-medium-desktop-material");
    assert!(
        !material.contains("data-tint") && !material.contains("class=\"ds-frame\""),
        "the bare material is unmarked: {material}"
    );
    assert!(
        !tile.contains("data-tint") && !tile.contains("class=\"ds-frame\""),
        "a tile lays no second gradient on the center's tinted popover: {tile}"
    );
    let space = html("frame-medium-desktop-space");
    assert!(space.contains("data-tint=\"space\""), "{space}");
    assert!(space.contains("class=\"ds-frame\""), "{space}");
}

/// The arc runs clockwise from twelve as far as the level (a full level is the whole circle),
/// its tone follows the level unless charging, a charging ring leaves a gap at twelve for its
/// bolt, and it reports its value as a progress bar.
#[test]
fn the_battery_draws_its_level_and_tone() {
    let cases = [
        (
            "battery-8",
            "M50.00 4.65A45.35 45.35 0 0 1 71.85 10.26",
            "low",
            "8",
        ),
        (
            "battery-45",
            "M50.00 4.65A45.35 45.35 0 0 1 64.01 93.13",
            "ok",
            "45",
        ),
        (
            "battery-100",
            "M50.00 4.65A45.35 45.35 0 0 1 50.00 95.35A45.35 45.35 0 0 1 50.00 4.65Z",
            "ok",
            "100",
        ),
        (
            "battery-charging-15",
            "M61.35 6.09A45.35 45.35 0 0 1 90.81 30.23",
            "ok",
            "15",
        ),
    ];
    for (name, arc, tone, now) in cases {
        let html = html(name);
        assert!(html.contains(&format!("d=\"{arc}\"")), "{name}: {html}");
        assert!(
            html.contains(&format!("data-tone=\"{tone}\"")),
            "{name}: {html}"
        );
        assert!(
            html.contains(&format!("aria-valuenow=\"{now}\"")),
            "{name}: {html}"
        );
        assert!(!html.contains("a-bump"), "{name}: nothing bumps on mount");
    }
    let charging = html("battery-charging-15");
    assert!(charging.contains("data-power=\"charging\""), "{charging}");
    assert!(charging.contains("ds-battery-bolt"), "{charging}");
    assert!(
        charging.contains("d=\"M61.35 6.09A45.35 45.35 0 1 1 38.65 6.09\""),
        "the track leaves the bolt's gap: {charging}"
    );
    assert!(!html("battery-45").contains("ds-battery-bolt"));
}

/// A ring's first frame is its level: the arc and the bolt are drawn, and the true level is
/// given to assistive technology, with the low red already chosen from it.
#[test]
fn a_battery_first_draws_at_its_level() {
    let first = render(|| {
        desktop(
            Theme::Light,
            rsx! {
                BatteryRing { state: at(80), label: "Mouse" }
                BatteryRing { state: charging(150), label: "This computer" }
                Label { text: percent_text(Fraction(930)) }
            },
        )
    });
    assert!(first.contains("ds-progress-fill"), "{first}");
    assert!(first.contains("ds-battery-bolt"), "{first}");
    assert!(first.contains("aria-valuenow=\"8\""), "{first}");
    assert!(first.contains("aria-valuenow=\"15\""), "{first}");
    assert!(first.contains("data-tone=\"low\""), "{first}");
    assert!(
        first.contains(">93%<"),
        "the figure shows the level: {first}"
    );
}
