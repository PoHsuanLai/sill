//! The desktop widget layer on the daemon's side: one surface per output while `widgets.desktop` is
//! `On`, closed when it turns `Off`.

use sill_services::env::Dirs;

use shell_host::{HostCtx, HostRoot, OutputInfo, SurfaceId};
use sill_settings::{DesktopWidgets, ShellFile};
use sill_surfaces::surfaces::desktop_widgets::{DesktopWidgetsProps, PlacementFile, desktop_layer};
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
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
    dirs: &Dirs,
    root: impl Fn(HostRoot) -> HostRoot,
) {
    let desktop = settings.borrow().widgets.desktop;
    let rows: Vec<_> = outputs
        .outputs
        .iter()
        .map(|(info, s)| (info.clone(), s.desktop_widgets))
        .collect();
    for action in layer_actions(desktop, &rows) {
        match action {
            LayerAction::Spawn(info) => spawn(ctx, outputs, &root, dirs, info),
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
    root: &impl Fn(HostRoot) -> HostRoot,
    dirs: &Dirs,
    info: OutputInfo,
) {
    let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
    let props = DesktopWidgetsProps {
        output: info.clone(),
    };
    let mut made = root(HostRoot::with_props(sill_surfaces::DesktopWidgets, props));
    made = made.context(PlacementFile::in_dirs(dirs));
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

/// The desktop layer's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    desktop_layer(info).namespace
}

#[cfg(test)]
mod tests {
    use shell_host::{OutputId, OutputInfo, Rect, Scale, SurfaceId};
    use sill_settings::DesktopWidgets;

    use super::{LayerAction, layer_actions};

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
}
