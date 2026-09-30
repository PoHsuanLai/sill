//! What the switcher hands quire's `AppSwitcher`: the tiles, the metrics, and the way between
//! quire's `AppKey` and the machine's indices. Pure.

use super::apps::SwitcherApp;
use ds::{IconSource, Px};
use ds_shell::{AppKey, SwitcherApp as Tile, SwitcherMetrics};
use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_settings::SwitcherSettings;

/// One tile per app, in order: the key is the app id, the name the desktop entry's `Name` (the
/// id when it has no entry), the icon from `icon` (the caller's resolver, at the cell's icon
/// size), no plate, `Present`.
pub fn tiles(
    apps: &[SwitcherApp],
    index: &AppIndex,
    icon: impl Fn(&AppId) -> IconSource,
) -> Vec<Tile> {
    apps.iter()
        .map(|app| Tile::new(app.app.0.clone(), name_of(&app.app, index), icon(&app.app)))
        .collect()
}

/// The entry's `Name`, else the id.
fn name_of(app: &AppId, index: &AppIndex) -> String {
    index
        .get(app)
        .map(|entry| entry.name.trim())
        .filter(|name| !name.is_empty())
        .map_or_else(|| app.0.clone(), str::to_owned)
}

/// quire's metrics from `switcher.*` (`icon_size_px`, `cell_size_px`, `cell_gap_px`,
/// `overflow_min_icon_px`).
pub fn metrics_of(settings: &SwitcherSettings) -> SwitcherMetrics {
    SwitcherMetrics {
        icon: px(settings.icon_size_px),
        cell: px(settings.cell_size_px),
        gap: px(settings.cell_gap_px),
        min_icon: px(settings.overflow_min_icon_px),
    }
}

fn px(value: ds_settings::Px) -> Px {
    Px(f32::from(value.0))
}

/// quire's key for `app`.
pub fn key_of(app: &AppId) -> AppKey {
    AppKey(app.0.clone())
}

/// The machine's index of `key` in `apps` (what `onhover` and `onactivate` become).
pub fn index_of(key: &AppKey, apps: &[SwitcherApp]) -> Option<usize> {
    apps.iter().position(|app| app.app.0 == key.0)
}

/// The output's width in logical pixels, for quire's overflow fit.
pub fn output_width(width: i32) -> Option<Px> {
    u16::try_from(width)
        .ok()
        .filter(|&width| width > 0)
        .map(|width| Px(f32::from(width)))
}

#[cfg(test)]
mod tests {
    use crate::switcher::apps::{AppShown, SwitcherApp};
    use ds::{IconSource, Px};
    use ds_shell::{AppKey, SwitcherMetrics};
    use sill_launcher::AppId;
    use sill_model::apps::AppIndex;
    use sill_settings::SwitcherSettings;

    use super::{index_of, key_of, metrics_of, output_width, tiles};

    fn app(id: &str) -> SwitcherApp {
        SwitcherApp {
            app: AppId(id.to_owned()),
            windows: Vec::new(),
            shown: AppShown::Visible,
        }
    }

    #[test]
    fn tiles_keys_and_metrics() {
        let apps = [app("foot"), app("org.mozilla.firefox")];
        let icon = |_: &AppId| IconSource::Glyph(ds::Icon::ArrowRight);
        let row = tiles(&apps, &AppIndex::default(), icon);
        assert_eq!(
            row.iter().map(|t| t.key.clone()).collect::<Vec<_>>(),
            vec![key_of(&apps[0].app), key_of(&apps[1].app)]
        );
        assert_eq!(row[1].name, "org.mozilla.firefox", "no entry: the id");
        assert_eq!(index_of(&AppKey("foot".to_owned()), &apps), Some(0));
        assert_eq!(index_of(&AppKey("mail".to_owned()), &apps), None);
        assert_eq!(
            metrics_of(&SwitcherSettings::default()),
            SwitcherMetrics::default()
        );
        assert_eq!(output_width(1920), Some(Px(1920.0)));
        assert_eq!(output_width(0), None);
    }
}
