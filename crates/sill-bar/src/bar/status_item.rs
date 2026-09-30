//! One icon status item: a quire `MenuBarItem`, expanded while its menu is
//! open (design/20 §1.1), sized by the bar's status keys through `ds::style::tokens::status::StatusMetrics`.

use dioxus::prelude::*;
use ds::components::controls::button_model::ImagePosition;
use ds::host::measure::use_rect;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::tokens::status::StatusMetrics;
use ds_shell::prelude::*;
use sill_settings::{BarGlyphSize, BarSettings};

use super::menus::BarMenu;
use super::title::{shown_of, use_title_pointer};

/// The status items' box and glyph from `bar.status_icon_box_px`, `bar.status_glyph_px` and
/// `bar.glyph_size_policy`: under `IconSizeBar22` the glyph fills the box.
pub(crate) fn status_metrics(bar: &BarSettings) -> StatusMetrics {
    let glyph = match bar.glyph_size_policy {
        BarGlyphSize::StatusIcon16 => bar.status_glyph_px.0,
        BarGlyphSize::IconSizeBar22 => bar.status_icon_box_px.0,
    };
    StatusMetrics {
        box_size: Px(f32::from(bar.status_icon_box_px.0)),
        glyph: Px(f32::from(glyph)),
    }
}

/// The status item for `menu`, drawn as `icon` (a glyph, or a layered status glyph for the Wi-Fi,
/// battery, volume and Bluetooth items) with `tooltip`. The button draws its own pill
/// while expanded.
#[component]
pub(crate) fn StatusIcon(
    menu: BarMenu,
    #[props(into)] icon: IconSource,
    tooltip: String,
) -> Element {
    let probe = use_rect();
    let pointer = use_title_pointer(menu, probe);
    rsx! {
        MenuBarItem {
            common: Common { mounted: Some(EventHandler::new(move |event| probe.on_mounted(event))), ..Common::default() },
            image: ImagePosition::Only,
            icon,
            label: menu.label().to_owned(),
            title: Some(tooltip),
            shown: shown_of(menu),
            pointer,
            onclick: move |_| {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::status_metrics;
    use ds::prelude::*;
    use sill_settings::{BarGlyphSize, BarSettings, Px as KeyPx};

    #[test]
    fn the_glyph_policy_picks_the_glyph_or_the_box() {
        // (box, glyph, policy, the metrics' box and glyph)
        const CASES: &[(u16, u16, BarGlyphSize, f32, f32)] = &[
            (22, 16, BarGlyphSize::StatusIcon16, 22.0, 16.0),
            (22, 16, BarGlyphSize::IconSizeBar22, 22.0, 22.0),
            (26, 18, BarGlyphSize::StatusIcon16, 26.0, 18.0),
            (26, 18, BarGlyphSize::IconSizeBar22, 26.0, 26.0),
        ];
        for &(size, glyph, policy, want_box, want_glyph) in CASES {
            let bar = BarSettings {
                status_icon_box_px: KeyPx(size),
                status_glyph_px: KeyPx(glyph),
                glyph_size_policy: policy,
                ..BarSettings::default()
            };
            let metrics = status_metrics(&bar);
            assert_eq!(
                (metrics.box_size, metrics.glyph),
                (Px(want_box), Px(want_glyph)),
                "{size} {glyph} {policy:?}"
            );
        }
        assert_eq!(
            status_metrics(&BarSettings::default()).style_attr(),
            "--bar-status-box:22px;--bar-status-glyph:16px;"
        );
    }
}
