//! One icon status item: a quire `MenuBarItem`, expanded while its menu is
//! open (design/20 §1.1), sized by the bar's status keys through `ds::style::tokens::status::StatusMetrics`.

use dioxus::prelude::*;
use ds::components::controls::button_model::ImagePosition;
use ds::host::measure::use_rect;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::tokens::status::StatusMetrics;
use sill_settings::BarSettings;
use sill_shell_kit::prelude::*;

use super::menus::BarMenu;
use super::title::{shown_of, use_title_pointer};

/// The status items' box and glyph from `bar.status_icon_box_px` and `bar.status_glyph_px`.
pub(crate) fn status_metrics(bar: &BarSettings) -> StatusMetrics {
    StatusMetrics {
        box_size: Px(f32::from(bar.status_icon_box_px.0)),
        glyph: Px(f32::from(bar.status_glyph_px.0)),
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
    use sill_settings::{BarSettings, Px as KeyPx};

    #[test]
    fn the_metrics_are_the_box_and_the_glyph() {
        // (box, glyph)
        const CASES: &[(u16, u16)] = &[(22, 16), (26, 18)];
        for &(size, glyph) in CASES {
            let bar = BarSettings {
                status_icon_box_px: KeyPx(size),
                status_glyph_px: KeyPx(glyph),
                ..BarSettings::default()
            };
            let metrics = status_metrics(&bar);
            assert_eq!(
                (metrics.box_size, metrics.glyph),
                (Px(f32::from(size)), Px(f32::from(glyph))),
                "{size} {glyph}"
            );
        }
        assert_eq!(
            status_metrics(&BarSettings::default()).style_attr(),
            "--bar-status-box:22px;--bar-status-glyph:16px;"
        );
    }
}
