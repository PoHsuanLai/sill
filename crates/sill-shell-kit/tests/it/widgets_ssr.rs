//! The widget parts as markup (design/04-COMPONENTS.md "Widgets"):
//! the frame in each size on the desktop and as a tile, the clock face in both looks and
//! phases, and the battery at three levels and charging. Each golden is
//! `tests/snapshots/widgets/<name>.html`; each lints clean and every `ds-` class in it is
//! styled by the stylesheet.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it widgets_ssr` rewrites the goldens.

use crate::support::golden;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::icon::render::Glyph;
use ds_lint::{LintConfig, markup};
use sill_shell_kit::battery::ring::percent_text;
use sill_shell_kit::clock::face::ClockFace;
use sill_shell_kit::clock::kind::{ClockLook, ClockTime, DayPhase, Seconds};
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

const TEN_TO_TEN: ClockTime = ClockTime {
    hour: 9,
    minute: 50,
    second: Seconds::Hidden,
};

const LATE: ClockTime = ClockTime {
    hour: 23,
    minute: 5,
    second: Seconds::Shown(42),
};

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
    ("clock-analog-day", || {
        desktop(
            Theme::Light,
            rsx! { ClockFace { time: TEN_TO_TEN, label: "Taipei" } },
        )
    }),
    ("clock-analog-small-frame", || {
        desktop(
            Theme::Light,
            rsx! { WidgetFrame { size: WidgetSize::Small, ClockFace { time: LATE, label: "Taipei" } } },
        )
    }),
    ("clock-analog-night-seconds", || {
        desktop(
            Theme::Light,
            rsx! { ClockFace { time: LATE, phase: DayPhase::Night, label: "London" } },
        )
    }),
    ("clock-digital-day", || {
        desktop(
            Theme::Dark,
            rsx! { ClockFace { time: TEN_TO_TEN, look: ClockLook::Digital, label: "Taipei" } },
        )
    }),
    ("clock-digital-night-seconds", || {
        desktop(
            Theme::Light,
            rsx! { ClockFace { time: LATE, phase: DayPhase::Night, look: ClockLook::Digital, label: "London" } },
        )
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

#[test]
fn every_widget_part_matches_its_golden() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|(name, make)| {
            golden::check(&format!("widgets/{name}.html"), &render(*make)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_part_lints_clean_and_every_class_is_styled() {
    let sheet = sill_shell_kit::stylesheet();
    let mut failures = Vec::new();
    for (name, make) in CASES {
        let html = render(*make);
        for offence in markup(&html, sheet, &LintConfig::new(&sill_shell_kit::kits())) {
            failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
        }
        for class in html
            .split("class=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .flat_map(str::split_whitespace)
            .filter(|class| class.starts_with("ds-"))
        {
            let needle = format!(".{class}");
            let styled = sheet.match_indices(&needle).any(|(at, _)| {
                !sheet[at + needle.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            });
            if !styled {
                failures.push(format!("{name}: .{class} is not styled"));
            }
        }
    }
    failures.dedup();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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

/// The dial says its phase (its face and ink follow it); twelve numerals stand round it; the
/// second hand and its pin are drawn only when the seconds are shown; a digital face wears the
/// sun by day and the moon by night.
#[test]
fn the_dial_keeps_its_pairs_and_its_second_hand_only_when_shown() {
    let day = html("clock-analog-day");
    assert!(day.contains("data-theme=\"light\""), "{day}");
    assert!(day.contains("data-phase=\"day\""), "{day}");
    assert!(!day.contains("ds-clock-second"), "{day}");
    assert!(!day.contains("ds-clock-pin"), "{day}");
    assert_eq!(
        day.matches("class=\"ds-clock-numeral\"").count(),
        12,
        "{day}"
    );
    assert!(
        day.contains("rotate(295 50 50)"),
        "the hour hand at ten to ten: {day}"
    );
    let night = html("clock-analog-night-seconds");
    assert!(night.contains("data-theme=\"light\""), "{night}");
    assert!(night.contains("data-phase=\"night\""), "{night}");
    assert!(night.contains("ds-clock-second"), "{night}");
    assert!(night.contains("ds-clock-pin"), "{night}");
    assert!(
        night.contains("rotate(252 50 50)"),
        "the second hand at 42 s: {night}"
    );
    let digital_night = html("clock-digital-night-seconds");
    assert!(digital_night.contains(">23:05:42<"), "{digital_night}");
    assert!(digital_night.contains("ds-clock-moon"), "{digital_night}");
    let digital_day = html("clock-digital-day");
    assert!(digital_day.contains(">09:50<"), "{digital_day}");
    assert!(digital_day.contains("ds-clock-sun"), "{digital_day}");
    assert!(
        !day.contains("ds-clock-sun"),
        "an analog face wears no mark"
    );
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
