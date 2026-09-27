//! The desktop widget layer on the daemon's side (FINDINGS "M10 freeze"): one surface per output
//! while `widgets.desktop` is `On` (closed when it turns `Off`), and `sill widgets
//! toggle-desktop`, which flips the key in `settings.toml` (the file stays the one truth; the
//! watch brings the change back as a `ShellMsg::Settings` too, and the reply publishes it at
//! once so the layer follows before the watch has fired).

use std::path::Path;

use shell_host::{HostCtx, OutputInfo, Root, SurfaceId};
use sill_ipc::{Failure, FailureKind, Response, WidgetsRequest};
use sill_settings::{DesktopWidgets, ShellFile};
use sill_surfaces::surfaces::desktop_widgets::{DesktopWidgetsProps, PlacementFile, desktop_layer};
use tokio::sync::watch;

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// What one output's desktop layer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerAction {
    /// Spawn it on this output.
    Spawn(OutputInfo),
    /// Close this surface.
    Close(SurfaceId),
}

/// What `rows` (each output with its layer, if any) need for `desktop`: a layer on each output
/// without one while `On`, none anywhere while `Off`.
pub fn layer_actions(
    desktop: DesktopWidgets,
    rows: &[(OutputInfo, Option<SurfaceId>)],
) -> Vec<LayerAction> {
    rows.iter()
        .filter_map(|(info, layer)| match (desktop, layer) {
            (DesktopWidgets::On, None) => Some(LayerAction::Spawn(info.clone())),
            (DesktopWidgets::Off, Some(id)) => Some(LayerAction::Close(*id)),
            (DesktopWidgets::On, Some(_)) | (DesktopWidgets::Off, None) => None,
        })
        .collect()
}

/// Spawn the desktop layer on every output that lacks one while `widgets.desktop` is `On`, and
/// close every one while it is `Off`. Its root gets the placements' file
/// (`$XDG_STATE_HOME/sill/desktop-widgets.json`).
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    settings: &watch::Sender<ShellFile>,
    root: impl Fn(Root) -> Root,
) {
    let desktop = settings.borrow().widgets.desktop;
    let rows: Vec<_> = outputs
        .outputs
        .iter()
        .map(|(info, s)| (info.clone(), s.desktop_widgets))
        .collect();
    for action in layer_actions(desktop, &rows) {
        match action {
            LayerAction::Spawn(info) => spawn(ctx, outputs, &root, info),
            LayerAction::Close(id) => {
                ctx.close(id);
                if let Some((_, surfaces)) = outputs.row_with(id) {
                    surfaces.clear(id);
                }
            }
        }
    }
}

/// Spawn `info`'s layer, shown (it is on `Bottom`, under every window).
fn spawn(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: &impl Fn(Root) -> Root,
    info: OutputInfo,
) {
    let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
    let props = DesktopWidgetsProps {
        output: info.clone(),
    };
    let mut made = root(Root::with_props(sill_surfaces::DesktopWidgets, props));
    if let Some(file) = PlacementFile::default_path() {
        made = made.context(file);
    }
    let config = desktop_layer(&info);
    let namespace = config.namespace.clone();
    surfaces.desktop_widgets = match ctx.spawn_layer(config, made) {
        Ok(id) => Some(id),
        Err(error) => {
            eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name);
            None
        }
    };
    outputs.set(info, surfaces);
}

/// `widgets.desktop` flipped.
pub fn flipped(now: DesktopWidgets) -> DesktopWidgets {
    match now {
        DesktopWidgets::Off => DesktopWidgets::On,
        DesktopWidgets::On => DesktopWidgets::Off,
    }
}

/// `sill widgets ...`. `ToggleDesktop` loads `settings.toml` from `dir`, flips
/// `widgets.desktop` with [`flipped`], saves it and publishes it on `settings`; `Edit` and
/// `EditClose` write Edit Widgets' link (`crate::edit_widgets`).
pub fn dispatch(
    dir: Option<&Path>,
    settings: &watch::Sender<ShellFile>,
    outputs: &Outputs,
    links: Option<&Links>,
    request: WidgetsRequest,
) -> Response {
    let edit = links.map(|links| &links.edit_widgets);
    match request {
        WidgetsRequest::ToggleDesktop => toggle_desktop(dir, settings),
        WidgetsRequest::Edit => crate::edit_widgets::open(outputs, edit),
        WidgetsRequest::EditClose => crate::edit_widgets::close(edit),
    }
}

/// Flip `widgets.desktop` in `dir`'s `settings.toml` and publish it.
fn toggle_desktop(dir: Option<&Path>, settings: &watch::Sender<ShellFile>) -> Response {
    let Some(dir) = dir else {
        return failed(
            FailureKind::Internal,
            "no settings directory (XDG_CONFIG_HOME and HOME are both unset)".to_owned(),
        );
    };
    let mut file = sill_settings::load(dir);
    file.widgets.desktop = flipped(file.widgets.desktop);
    if let Err(error) = sill_settings::save(dir, &file) {
        return failed(
            FailureKind::Internal,
            format!("saving the settings: {error}"),
        );
    }
    settings.send_replace(file);
    Response::Done
}

fn failed(kind: FailureKind, message: String) -> Response {
    Response::Failed(Failure { kind, message })
}

/// The desktop layer's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    desktop_layer(info).namespace
}

#[cfg(test)]
mod tests {
    use shell_host::{OutputId, OutputInfo, Rect, Scale, SurfaceId};
    use sill_ipc::{Response, WidgetsRequest};
    use sill_settings::{DesktopWidgets, ShellFile};
    use tokio::sync::watch;

    use super::{LayerAction, dispatch, flipped, layer_actions};
    use crate::outputs::Outputs;

    fn output(id: u32) -> OutputInfo {
        OutputInfo {
            id: OutputId(id),
            name: Some(format!("DP-{id}")),
            description: None,
            logical: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale: Scale::ONE,
            refresh_mhz: 60_000,
        }
    }

    #[test]
    fn toggling_flips_the_key() {
        assert_eq!(flipped(DesktopWidgets::Off), DesktopWidgets::On);
        assert_eq!(flipped(DesktopWidgets::On), DesktopWidgets::Off);
    }

    #[test]
    fn a_layer_per_output_while_on_and_none_while_off() {
        let rows = [(output(1), None), (output(2), Some(SurfaceId(7)))];
        let cases = [
            (DesktopWidgets::On, vec![LayerAction::Spawn(output(1))]),
            (DesktopWidgets::Off, vec![LayerAction::Close(SurfaceId(7))]),
        ];
        for (desktop, want) in cases {
            assert_eq!(layer_actions(desktop, &rows), want, "{desktop:?}");
        }
        assert_eq!(
            layer_actions(DesktopWidgets::Off, &[(output(1), None)]),
            Vec::new()
        );
    }

    #[test]
    fn toggle_desktop_flips_saves_and_publishes() {
        let dir = std::env::temp_dir().join(format!("sill-widgets-toggle-{}", std::process::id()));
        let (tx, rx) = watch::channel(ShellFile::default());
        let outputs = Outputs::default();
        for want in [DesktopWidgets::On, DesktopWidgets::Off] {
            let response = dispatch(
                Some(&dir),
                &tx,
                &outputs,
                None,
                WidgetsRequest::ToggleDesktop,
            );
            assert_eq!(response, Response::Done);
            assert_eq!(sill_settings::load(&dir).widgets.desktop, want);
            assert_eq!(rx.borrow().widgets.desktop, want);
        }
        let _ = std::fs::remove_dir_all(dir);
        assert!(matches!(
            dispatch(None, &tx, &outputs, None, WidgetsRequest::ToggleDesktop),
            Response::Failed(_)
        ));
    }
}
