//! What the switcher hands quire's `AppSwitcher`: the tiles, the metrics, and the way between
//! quire's `AppKey` and the machine's indices. Pure.

use ds::{AppKey, IconSource, Px, SwitcherApp as Tile, SwitcherMetrics};
use sill_launcher::AppId;
use sill_services::apps::AppIndex;
use sill_services::switcher::SwitcherApp;
use sill_settings::SwitcherSettings;

/// One tile per app, in order: the key is the app id, the name the desktop entry's `Name` (the
/// id when it has no entry), the icon from `icon` (the caller's resolver, at the cell's icon
/// size), no plate, `Present`.
pub fn tiles(
    apps: &[SwitcherApp],
    index: &AppIndex,
    icon: impl Fn(&AppId) -> IconSource,
) -> Vec<Tile> {
    todo!(
        "M11 fill: c-switcher ({} apps, {} entries, {:?})",
        apps.len(),
        index.apps.len(),
        apps.first().map(|app| icon(&app.app))
    )
}

/// quire's metrics from `switcher.*` (`icon_size_px`, `cell_size_px`, `cell_gap_px`,
/// `overflow_min_icon_px`).
pub fn metrics_of(settings: &SwitcherSettings) -> SwitcherMetrics {
    todo!("M11 fill: c-switcher ({settings:?})")
}

/// quire's key for `app`.
pub fn key_of(app: &AppId) -> AppKey {
    AppKey(app.0.clone())
}

/// The machine's index of `key` in `apps` (what `onhover` and `onactivate` become).
pub fn index_of(key: &AppKey, apps: &[SwitcherApp]) -> Option<usize> {
    todo!("M11 fill: c-switcher ({key:?} {} apps)", apps.len())
}

/// The output's width in logical pixels, for quire's overflow fit.
pub fn output_width(width: i32) -> Option<Px> {
    todo!("M11 fill: c-switcher ({width})")
}

#[cfg(test)]
mod tests {
    use ds::{AppKey, IconSource, Px, SwitcherMetrics};
    use sill_launcher::AppId;
    use sill_services::apps::AppIndex;
    use sill_services::switcher::{AppShown, SwitcherApp};
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
    #[ignore = "M11 fill: c-switcher"]
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
