//! The shell-wide style inputs from the settings (quire's macOS polish pass): the shell type
//! scale (`ds::ShellMetrics`: the bar's items, text menus, the launcher, tooltips). Each surface
//! writes the metrics inline on an element around its `Ds` root and hands the root the material
//! stack from `ds_settings::Environment::material_stack`, so every card under it reads the keys;
//! absent, the keys' defaults apply anyway (quire's tokens fall back to them).

use ds::{FontWeight, Px, ShellMetrics};
use sill_settings::ShellFile;

/// The shell type scale from `bar.*`, `menus.*` and `launcher.*`.
pub(crate) fn shell_metrics(file: &ShellFile) -> ShellMetrics {
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

#[cfg(test)]
mod tests {
    use super::shell_metrics;
    use ds::ShellMetrics;
    use sill_settings::{Px, ShellFile};

    #[test]
    fn the_default_keys_are_quires_defaults() {
        assert_eq!(
            shell_metrics(&ShellFile::default()),
            ShellMetrics::default()
        );
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
