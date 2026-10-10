//! The desktop's own compositor, on the daemon's side: the connection it hands the shell, the
//! desktop arm that follows it on the shell host's connection, and what each note of its shell
//! protocol does.
//!
//! On a session without that compositor (COSMIC, KWin, a test) nothing here runs: the host
//! connects through the environment and the seat stays absent.
//!
//! The seat starts the arm at the first mapped surface that has a role: the arm binds beside the
//! host on that surface's connection, and every mapped surface after is registered with it (a
//! surface shown again has a new `wl_surface`, and `SurfaceMapped` comes again). Notes come back
//! as [`ShellMsg::Note`] and [`apply`] carries each out on the host thread.

use std::os::fd::OwnedFd;

use shell_host::{HostCtx, HostEvent, HostHandle, SurfaceId, WaylandRoute};
use sill_model::companion::CompanionCommand;
use sill_model::desktop::{Corner as ProtocolCorner, KeyGesture, ShellNote, ShellRole};
use sill_services::desktop_wl::quire::{ShellArm, WlSurface, role_of_surface, spawn_shell_arm};
use sill_ui_kit::links::Corner;
use tokio::runtime::Handle;

use super::events::Shell;
use super::msg::ShellMsg;
use crate::outputs::Outputs;

/// The route the host connects by: the socket the compositor handed over, else the environment.
pub fn host_route(handed: Option<OwnedFd>) -> WaylandRoute {
    handed.map_or(WaylandRoute::Environment, WaylandRoute::Handed)
}

/// Where the desktop arm runs from, once there is a connection to run it on.
struct Link {
    runtime: Handle,
    host: HostHandle<ShellMsg>,
}

/// The daemon's hold on the desktop arm.
#[derive(Default)]
pub struct ShellSeat {
    link: Option<Link>,
    arm: Option<ShellArm>,
}

impl std::fmt::Debug for ShellSeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSeat")
            .field("linked", &self.link.is_some())
            .field("running", &self.arm.is_some())
            .finish()
    }
}

impl ShellSeat {
    /// No compositor of ours: nothing starts.
    pub fn absent() -> ShellSeat {
        ShellSeat::default()
    }

    /// The host is on the connection the compositor handed over: the arm starts at the first
    /// mapped surface that has a role, and its notes reach `host`.
    pub fn linked(runtime: Handle, host: HostHandle<ShellMsg>) -> ShellSeat {
        ShellSeat {
            link: Some(Link { runtime, host }),
            arm: None,
        }
    }

    /// `surface` of the shell, with `role`, is on screen on `output`: start the arm if this is
    /// the first, and register it.
    fn mapped(&mut self, role: ShellRole, output: &str, surface: WlSurface) {
        let Some(link) = &self.link else {
            return;
        };
        if self.arm.is_none() {
            match spawn_shell_arm(&surface) {
                Ok(mut arm) => {
                    if let Some(mut notes) = arm.take_notes() {
                        let host = link.host.clone();
                        link.runtime.spawn(async move {
                            while let Some(note) = notes.recv().await {
                                if host.send(ShellMsg::Note(note)).is_err() {
                                    return;
                                }
                            }
                        });
                    }
                    self.arm = Some(arm);
                }
                Err(error) => {
                    eprintln!("sill: desktop: {error}; the shell protocol is not followed");
                    return;
                }
            }
        }
        let gone = self
            .arm
            .as_ref()
            .is_some_and(|arm| arm.register(role, output, surface).is_err());
        if gone {
            eprintln!("sill: desktop: the desktop arm has stopped");
            self.arm = None;
        }
    }
}

/// A host event that maps a shell surface registers it with the seat.
pub fn follow_mapped(
    seat: &mut ShellSeat,
    outputs: &Outputs,
    ctx: &HostCtx<ShellMsg>,
    event: &HostEvent<ShellMsg>,
) {
    let HostEvent::SurfaceMapped { id } = event else {
        return;
    };
    let Some((output, name)) = surface_named(outputs, *id) else {
        return;
    };
    let (Some(role), Some(surface)) = (role_of_surface(name), ctx.wl_surface(*id)) else {
        return;
    };
    seat.mapped(role, &output, surface);
}

/// The output's name and the `Surface::NAME` of the surface `id`.
fn surface_named(outputs: &Outputs, id: SurfaceId) -> Option<(String, &'static str)> {
    outputs.outputs.iter().find_map(|(info, surfaces)| {
        surfaces
            .spawned
            .iter()
            .find(|surface| surface.id == id)
            .map(|surface| (info.name.clone().unwrap_or_default(), surface.name))
    })
}

/// Carry out one note of the compositor's shell protocol on the host thread.
pub fn apply(shell: &Shell, note: ShellNote) {
    match note {
        ShellNote::HotCorner(corner) => hot_corner(shell, corner),
        ShellNote::Gesture(gesture) => key_gesture(shell, gesture),
        ShellNote::Killed => companion_stop(shell),
        ShellNote::Modifiers(_) => option_variants(),
        ShellNote::Switcher(_) => switcher_panel(),
        ShellNote::Overview(_) => mission_control_chrome(),
        ShellNote::Reveal { .. } => edge_reveal(),
        ShellNote::MenuFlash { .. } => menu_flash(),
        ShellNote::Bell(_) => bell(),
        ShellNote::Reopen(_) => reopen_in_dock(),
    }
}

/// The pointer pushed into a corner: the corner link's firing, which `corners::spawn_hits`
/// carries out as the corner's configured action. The compositor names no output, so the first
/// is used.
fn hot_corner(shell: &Shell, corner: ProtocolCorner) {
    let corner = match corner {
        ProtocolCorner::TopLeft => Corner::TopLeft,
        ProtocolCorner::TopRight => Corner::TopRight,
        ProtocolCorner::BottomLeft => Corner::BottomLeft,
        ProtocolCorner::BottomRight => Corner::BottomRight,
    };
    if let Some((info, _)) = shell.outputs.outputs.first() {
        shell.links.corners.hit(corner, info.id);
    }
}

/// The double-tap is the companion's tap; the hold gestures belong to the hold overlay.
fn key_gesture(shell: &Shell, gesture: KeyGesture) {
    match gesture {
        KeyGesture::Summon => {
            let focus = crate::companion::focus_of(&shell.services, &shell.links.launcher);
            let _companion_may_be_gone = shell
                .services
                .companion
                .control
                .send(CompanionCommand::Tap(focus));
        }
        KeyGesture::HoldStart | KeyGesture::HoldEnd | KeyGesture::HoldCancel => hold_overlay(),
    }
}

/// The kill chord: every run the companion has going is paused (casement-intents halts the
/// agent runs themselves).
fn companion_stop(shell: &Shell) {
    let _companion_may_be_gone = shell
        .services
        .companion
        .control
        .send(CompanionCommand::StopAll);
}

/// The hold overlay (a key held for its cheat sheet) has no surface yet. Lane P3-SILL.
fn hold_overlay() {}

/// The modifiers over a shell surface will switch the dock's and bar's Option variants and gate
/// the hot corners; keycap stays the source of truth for chords. Lane P3-SILL.
fn option_variants() {}

/// The compositor's switcher events (apps, shown, selected, hidden) will drive the switcher
/// panel, answered with `PickSwitcher` and `CancelSwitcher`; the panel still follows its own
/// machine. Lane P3-SW (casement side) and P3-SILL (the panel).
fn switcher_panel() {}

/// The overview's mode, progress and layout (`DesktopState::quire.shell().overview`) will draw
/// Mission Control's chrome. Lane P3-MC (casement side) and P3-SILL (the chrome).
fn mission_control_chrome() {}

/// An edge reveal will show or hide the bar and dock while a window is full screen.
/// Lane P3-SILL.
fn edge_reveal() {}

/// A menu item activated by a key will flash in the bar's menu. Lane P3-MN.
fn menu_flash() {}

/// An app's bell will bounce its dock icon or play the alert sound. Lane P3-SILL.
fn bell() {}

/// The compositor asks the dock to reopen an app with no windows; what reopening means (launch
/// it again, or bring its last window back) is the dock's decision. Lane P3-SILL.
fn reopen_in_dock() {}
