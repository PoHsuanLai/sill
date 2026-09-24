//! The shell-wide style inputs from the settings (quire's macOS polish pass): the shell type
//! scale (`ds::ShellMetrics`: the bar's items, text menus, the launcher, tooltips) and the
//! material stack (`ds::MaterialStack`: highlight, hairline, shadow strength, vibrancy). Each
//! surface writes the metrics inline on an element around its `Ds` root and hands the stack to
//! that root (`Ds { stack }`), so every card under it reads the keys; absent, the keys' defaults
//! apply anyway (quire's tokens fall back to them).

use ds::{Alpha, FontWeight, MaterialStack, Px, ShellMetrics};
use ds_settings::AppearanceSettings;
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

/// The material stack from `appearance.material_{highlight,hairline}_{light,dark}`,
/// `appearance.material_shadow_strength` and `appearance.material_vibrancy` (percent; quire
/// FINDINGS "macOS polish" names them). `AppearanceSettings` has no fields for them yet, so they
/// are read from the keys it keeps unknown (`extra`), each falling back to its default alone.
pub(crate) fn material_stack(appearance: &AppearanceSettings) -> MaterialStack {
    let key = |name: &str, default: Alpha| {
        appearance
            .extra
            .get(name)
            .and_then(|value| value.as_integer())
            .and_then(|percent| u16::try_from(percent.clamp(0, 100)).ok())
            .map_or(default, |percent| Alpha(percent * 10))
    };
    let stack = MaterialStack::default();
    MaterialStack {
        highlight_light: key("material_highlight_light", stack.highlight_light),
        highlight_dark: key("material_highlight_dark", stack.highlight_dark),
        hairline_light: key("material_hairline_light", stack.hairline_light),
        hairline_dark: key("material_hairline_dark", stack.hairline_dark),
        shadow_strength: key("material_shadow_strength", stack.shadow_strength),
        vibrancy: key("material_vibrancy", stack.vibrancy),
    }
}

#[cfg(test)]
mod tests {
    use super::{material_stack, shell_metrics};
    use ds::{Alpha, MaterialStack, ShellMetrics};
    use ds_settings::AppearanceSettings;
    use sill_settings::{Px, ShellFile};

    #[test]
    fn the_default_keys_are_quires_defaults() {
        assert_eq!(
            shell_metrics(&ShellFile::default()),
            ShellMetrics::default()
        );
        assert_eq!(
            material_stack(&AppearanceSettings::default()),
            MaterialStack::default()
        );
    }

    #[test]
    fn a_key_moves_its_own_input() {
        let mut file = ShellFile::default();
        file.menus.item_height_px = Px(24);
        let written = shell_metrics(&file).style_attr();
        assert!(written.contains("--shell-menu-row-h:24px;"), "{written}");
        assert!(written.contains("--shell-menu-font:13px;"), "{written}");
        let mut appearance = AppearanceSettings::default();
        appearance
            .extra
            .insert("material_vibrancy".to_owned(), 0.into());
        appearance
            .extra
            .insert("material_hairline_dark".to_owned(), 250.into());
        let stack = material_stack(&appearance);
        assert_eq!(stack.vibrancy, Alpha(0));
        assert_eq!(stack.hairline_dark, Alpha(1000), "clamped to 100 %");
        assert_eq!(
            stack.highlight_light,
            MaterialStack::default().highlight_light
        );
    }
}
