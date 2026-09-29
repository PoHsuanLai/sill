//! Edit Widgets on the daemon's side: one hidden surface per output, kept warm like the
//! notification center; and the writer its root is given, which saves the widget keys into `settings.toml` (`widgets.center`,
//! `widgets.desktop_widgets`, and `widgets.desktop`, which an add to the desktop turns on)
//! (publishing them at once, as `toggle-desktop` does, so the desktop layer and the center follow
//! before the watch fires) and the output's places into `desktop-widgets.json`.

use sill_services::env::Dirs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use shell_host::{HostCtx, OutputInfo, Root};
use sill_settings::ShellFile;
use sill_surfaces::surfaces::desktop_widgets::PlacementFile;
use sill_surfaces::surfaces::edit_widgets::{
    EditWidgetsProps, WidgetsWrite, WidgetsWriter, edit_widgets_layer,
};
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden Edit Widgets surface on every output whose row lacks one; each root is built
/// by `root` (the daemon's `surface_root`, which adds the link and the writer) and given the
/// places' file in `dirs`.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    dirs: &Dirs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.edit_widgets.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let props = EditWidgetsProps {
            output: info.clone(),
        };
        let mut made = root(Root::with_props(sill_surfaces::EditWidgets, props));
        made = made.context(PlacementFile::in_dirs(dirs));
        surfaces.edit_widgets =
            crate::daemon::surfaces::spawn_hidden(ctx, edit_widgets_layer(&info), made, &info);
        outputs.set(info, surfaces);
    }
}

/// The surface's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    edit_widgets_layer(info).namespace
}

/// The writer the surfaces are given: [`write`] into `dir` (nothing but a log line without
/// one) and the default places' file.
pub fn writer(
    dir: Option<PathBuf>,
    places: PlacementFile,
    settings: watch::Sender<ShellFile>,
) -> WidgetsWriter {
    WidgetsWriter(Arc::new(move |change: WidgetsWrite| {
        let Some(dir) = dir.as_deref() else {
            eprintln!("sill: no settings directory; Edit Widgets saved nothing");
            return;
        };
        if let Err(error) = write(dir, &settings, Some(&places), change) {
            eprintln!("sill: Edit Widgets: {error}");
        }
    }))
}

/// Save `change`'s keys into `dir`'s `settings.toml`, keeping every other key, publish the
/// file on `settings`, and save its places in `places` when there is a file.
pub fn write(
    dir: &Path,
    settings: &watch::Sender<ShellFile>,
    places: Option<&PlacementFile>,
    change: WidgetsWrite,
) -> Result<(), String> {
    let mut file = sill_settings::load(dir);
    file.widgets.center = change.center;
    file.widgets.desktop_widgets = change.desktop;
    file.widgets.desktop = change.shown;
    sill_settings::save(dir, &file).map_err(|error| format!("saving the settings: {error}"))?;
    settings.send_replace(file);
    if let Some(places) = places {
        places
            .save(&change.output, &change.places)
            .map_err(|error| format!("saving {}: {error}", places.0.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sill_settings::{DesktopWidgets, ShellFile, WidgetKind};
    use sill_surfaces::surfaces::desktop_widgets::{GridSlot, OutputKey, Placement, PlacementFile};
    use sill_surfaces::surfaces::edit_widgets::WidgetsWrite;
    use tokio::sync::watch;

    use super::write;

    #[test]
    fn an_edit_writes_the_widget_keys_and_the_outputs_places() {
        let dir = std::env::temp_dir().join(format!("sill-edit-widgets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let places = PlacementFile(dir.join("state").join("desktop-widgets.json"));
        let (tx, rx) = watch::channel(ShellFile::default());
        let mut before = ShellFile::default();
        before.widgets.world_clocks = "Europe/London".to_owned();
        sill_settings::save(&dir, &before).unwrap_or_else(|e| panic!("{e}"));
        let clock = Placement {
            kind: WidgetKind::WorldClock,
            slot: GridSlot { column: 2, row: 0 },
        };
        let change = WidgetsWrite {
            center: vec![WidgetKind::UpNext],
            desktop: vec![WidgetKind::Calendar, WidgetKind::WorldClock],
            shown: DesktopWidgets::On,
            output: OutputKey::Named("DP-1".to_owned()),
            places: vec![clock],
        };
        write(&dir, &tx, Some(&places), change).unwrap_or_else(|e| panic!("{e}"));
        let saved = sill_settings::load(&dir).widgets;
        assert_eq!(saved.center, vec![WidgetKind::UpNext]);
        assert_eq!(
            saved.desktop_widgets,
            vec![WidgetKind::Calendar, WidgetKind::WorldClock]
        );
        assert_eq!(
            saved.desktop,
            DesktopWidgets::On,
            "an add to the desktop shows it"
        );
        assert_eq!(
            saved.world_clocks, "Europe/London",
            "every other key is kept"
        );
        assert_eq!(rx.borrow().widgets.desktop, DesktopWidgets::On);
        assert_eq!(
            places.load(&OutputKey::Named("DP-1".to_owned())),
            vec![clock]
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
