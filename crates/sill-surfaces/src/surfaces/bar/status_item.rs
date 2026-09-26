//! One icon status item: a ds `IconButton{Status}` in a title slot, expanded while its menu is
//! open (design/20 §1.1), sized by the bar's status keys through `ds::StatusMetrics`.

use dioxus::prelude::*;
use ds::{Icon, IconButton, IconButtonVariant, PulseKey, Px, StatusMetrics, Switch, use_rect};
use sill_settings::{BarGlyphSize, BarSettings};

use super::menus::BarMenu;
use super::session::MenuBar;
use super::title::{SlotPill, TitleSlot};

/// The status items' box and glyph from `bar.status_icon_box_px`, `bar.status_glyph_px` and
/// `bar.glyph_size_policy` (CONSUMING.md §6): under `IconSizeBar22` the glyph fills the box.
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

/// The status item for `menu`, drawn as `icon` with `tooltip`. The button draws its own pill
/// while expanded, so the slot draws none. A `pulse` (the battery's low nudge, quire design/26
/// G11) plays on a wrapper around the button, which the slot still measures.
#[component]
pub(crate) fn StatusIcon(
    menu: BarMenu,
    icon: Icon,
    tooltip: String,
    #[props(default)] pulse: Option<PulseKey>,
) -> Element {
    let bar = use_context::<MenuBar>();
    let probe = use_rect();
    let expanded = if bar.is_open(menu) {
        Switch::On
    } else {
        Switch::Off
    };
    let (class, alias) = match pulse.and_then(PulseKey::attrs) {
        Some((anim, alias)) => (format!("sill-bar-status {anim}"), Some(alias)),
        None => ("sill-bar-status".to_owned(), None),
    };
    rsx! {
        TitleSlot { menu, probe, pill: SlotPill::Control,
            span { class, "data-pulse": alias,
                IconButton {
                    variant: IconButtonVariant::Status,
                    icon,
                    label: menu.label().to_owned(),
                    tooltip: Some(tooltip),
                    expanded: Some(expanded),
                    onclick: move |_| {},
                    mounted: move |event| probe.on_mounted(event),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::status_metrics;
    use ds::Px;
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
