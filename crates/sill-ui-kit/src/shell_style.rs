//! The shell-wide style inputs from the settings (quire's macOS polish pass): the shell type
//! scale (`ds::style::tokens::shell_type::ShellMetrics`: the bar's items, text menus, the launcher, tooltips). Each surface
//! writes the metrics inline on an element around its `Ds` root and hands the root the material
//! stack from `ds_settings::Environment::material_stack`, so every card under it reads the keys;
//! absent, the keys' defaults apply anyway (quire's tokens fall back to them).

use ds::prelude::*;
use ds::style::tokens::shell_type::{FontWeight, ShellMetrics};
use sill_settings::{PanelMaterial, ShellFile};

/// The shell type scale from `bar.*`, `menus.*` and `launcher.*`.
pub fn shell_metrics(file: &ShellFile) -> ShellMetrics {
    let px = |value: sill_settings::Px| Px(f32::from(value.0));
    let weight = |value: sill_settings::Count| FontWeight(value.0.clamp(100, 900));
    let mut metrics = ShellMetrics::default();
    metrics.bar.font = px(file.bar.item_font_px);
    metrics.bar.weight = weight(file.bar.item_font_weight);
    metrics.bar.item_height = px(file.bar.open_title_pill_height_px);
    metrics.bar.item_padding = px(file.bar.title_padding_px);
    metrics.bar.item_radius = px(file.bar.item_radius_px);
    metrics.menu.font = px(file.menus.font_px);
    metrics.menu.row = px(file.menus.item_height_px);
    metrics.menu.separator_margin = px(file.menus.separator_margin_px);
    metrics.menu.highlight_radius = px(file.menus.highlight_radius_px);
    metrics.launcher.field_font = px(file.launcher.field_font_px);
    metrics.launcher.field_weight = weight(file.launcher.field_font_weight);
    metrics.launcher.field_glyph = px(file.launcher.field_glyph_px);
    metrics.launcher.row_title = px(file.launcher.row_title_px);
    metrics.launcher.row_detail = px(file.launcher.row_detail_px);
    metrics.tip = px(file.menus.tooltip_font_px);
    metrics
}

/// `launcher.material` in quire's words (design/22 Sheet; design/20 Popover).
pub fn material(material: PanelMaterial) -> Material {
    match material {
        PanelMaterial::Sheet => Material::Sheet,
        PanelMaterial::Popover => Material::Popover,
    }
}

#[cfg(test)]
mod tests {
    use super::shell_metrics;
    use ds::style::tokens::control_size::ControlSize;
    use ds::style::tokens::shell_scale::SHELL_SCALE;
    use ds::style::tokens::shell_type::ShellMetrics;
    use ds_shell::tokens::control_center::CONTROL_CENTER;
    use sill_settings::{Px, ShellFile};

    #[test]
    fn the_default_keys_are_quires_defaults() {
        assert_eq!(
            shell_metrics(&ShellFile::default()),
            ShellMetrics::default()
        );
    }

    /// The size keys quire's `ShellMetrics` does not carry default to quire's size ladder
    /// (design/29 §13): a 24 bar of bar-scale items (22, not the Regular control, which is 28
    /// since the Tahoe ladder), Regular menu rows, and the control center's 320 wide grid with
    /// 10 of padding and gap.
    #[test]
    fn the_default_sizes_are_quires_ladder() {
        let file = ShellFile::default();
        let px = |value: Px| value.0;
        let regular = ControlSize::Regular.scale().height.0;
        let cases = [
            ("bar.height_px", px(file.bar.height_px), SHELL_SCALE.bar.0),
            (
                "bar.open_title_pill_height_px",
                px(file.bar.open_title_pill_height_px),
                SHELL_SCALE.item().0,
            ),
            (
                "bar.status_icon_box_px",
                px(file.bar.status_icon_box_px),
                SHELL_SCALE.item().0,
            ),
            (
                "menus.item_height_px",
                px(file.menus.item_height_px),
                regular,
            ),
            (
                "control_center.width_px",
                px(file.control_center.width_px),
                CONTROL_CENTER.width.0,
            ),
            (
                "control_center.grid_gap_px",
                px(file.control_center.grid_gap_px),
                CONTROL_CENTER.gap.0,
            ),
            (
                "control_center.grid_padding_px",
                px(file.control_center.grid_padding_px),
                CONTROL_CENTER.padding.0,
            ),
        ];
        for (key, ours, quires) in cases {
            assert_eq!(ours, quires, "{key}");
        }
    }

    #[test]
    fn a_key_moves_its_own_input() {
        let mut file = ShellFile::default();
        file.menus.item_height_px = Px(24);
        let written = shell_metrics(&file).style_attr();
        assert!(written.contains("--shell-menu-row-h:24px;"), "{written}");
        assert!(written.contains("--shell-menu-font:13px;"), "{written}");
    }
}
