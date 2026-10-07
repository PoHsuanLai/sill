//! ModuleTile's specimens: every state in each span, in both schemes, and the chevron's forms
//! (live, disabled with the tile, absent, open).

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::prelude::*;
use sill_shell_kit::control_center::module_tile_kind::TileSpan;
use sill_shell_kit::prelude::*;

/// One tile specimen.
#[derive(Props, Clone, PartialEq)]
pub struct TileCase {
    pub theme: Theme,
    pub value: Check,
    pub availability: Availability,
    pub span: TileSpan,
    pub detail: Detail,
    pub expanded: Shown,
}

/// Whether the specimen has a detail pane, and so a chevron.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    Chevron,
    Nothing,
}

pub const STATES: [(Check, Availability, &str); 3] = [
    (Check::Off, Availability::Enabled, "off"),
    (Check::On, Availability::Enabled, "on"),
    (Check::Off, Availability::Busy, "busy"),
];

pub const SPANS: [(TileSpan, &str); 2] = [(TileSpan::Half, "half"), (TileSpan::Full, "full")];

pub const THEMES: [(Theme, &str); 2] = [(Theme::Light, "light"), (Theme::Dark, "dark")];

/// Every state in each span and scheme, with a chevron: `tile-<state>-<span>-<scheme>`.
pub fn grid() -> Vec<(String, TileCase)> {
    let mut cases = Vec::new();
    for (theme, scheme) in THEMES {
        for (value, availability, word) in STATES {
            for (span, width) in SPANS {
                cases.push((
                    format!("tile-{word}-{width}-{scheme}"),
                    TileCase {
                        theme,
                        value,
                        availability,
                        span,
                        detail: Detail::Chevron,
                        expanded: Shown::Hidden,
                    },
                ));
            }
        }
    }
    cases
}

/// The chevron's other forms, in light: none, dimmed with a disabled tile, and open.
pub fn chevrons() -> Vec<(String, TileCase)> {
    let base = TileCase {
        theme: Theme::Light,
        value: Check::Off,
        availability: Availability::Enabled,
        span: TileSpan::Half,
        detail: Detail::Chevron,
        expanded: Shown::Hidden,
    };
    vec![
        (
            "tile-no-chevron".to_owned(),
            TileCase {
                detail: Detail::Nothing,
                ..base.clone()
            },
        ),
        (
            "tile-inert-chevron".to_owned(),
            TileCase {
                availability: Availability::Disabled,
                ..base.clone()
            },
        ),
        (
            "tile-open-chevron".to_owned(),
            TileCase {
                expanded: Shown::Visible,
                ..base
            },
        ),
    ]
}

/// A tile in a Popover root of its scheme.
pub fn tile(case: TileCase) -> Element {
    let on_detail = match case.detail {
        Detail::Chevron => Some(EventHandler::new(|_| {})),
        Detail::Nothing => None,
    };
    rsx! {
        Ds {
            appearance: Appearance { theme: case.theme, ..Appearance::default() },
            material: Material::Popover,
            stylesheet: Inject::Host,
            ModuleTile {
                glyph: Icon::Wifi,
                title: "Wi-Fi",
                status: "Home",
                value: case.value,
                availability: case.availability,
                span: case.span,
                onclick: |_| {},
                on_detail,
                expanded: case.expanded,
            }
        }
    }
}
