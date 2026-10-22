//! The desktop's own compositor, on the daemon's side: the connection it hands the shell, the
//! desktop arm that follows it on the shell host's connection, and what each note of its shell
//! protocol does.
//!
//! On a session without that compositor (KWin, a test) nothing here runs: the host
//! connects through the environment and the seat stays absent.
//!
//! The seat starts the arm at the first mapped surface that has a role: the arm binds beside the
//! host on that surface's connection, and every mapped surface after is registered with it (a
//! surface shown again has a new `wl_surface`, and `SurfaceMapped` comes again). Notes come back
//! as [`ShellMsg::Note`] and [`apply`] carries each out on the host thread.

use std::os::fd::OwnedFd;

use shell_host::{HostCtx, HostEvent, HostHandle, SurfaceId, WaylandRoute};
use sill_model::capture_pick::CaptureCommand;
use sill_model::companion::CompanionCommand;
use sill_model::desktop::target::{MinimiseTargets, OutputRects};
use sill_model::desktop::{
    Corner as ProtocolCorner, DesktopCommand, DesktopView, KeyGesture, ScreenEdge, ShellNote,
    ShellRole, Shown,
};
use sill_services::desktop_wl::quire::{ShellArm, WlSurface, spawn_shell_arm};
use sill_ui_kit::links::Corner;
use sill_ui_kit::surface::OutputPolicy;
use tokio::runtime::Handle;
use tokio::sync::watch;

use super::events::Shell;
use super::msg::ShellMsg;
use super::targets::{forward, nudge};
use crate::edge_reveal::FullScreenPrefs;
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
pub struct ShellSeat {
    link: Option<Link>,
    arm: Option<ShellArm>,
    /// The desktop's windows: the one channel the windows service writes.
    desktop: watch::Receiver<DesktopView>,
    /// The docks' icons and the targets the compositor holds for them.
    targets: Targets,
    /// What the compositor is told about full screen.
    prefs: Prefs,
}

/// The full-screen prefs wanted and what was last sent.
#[derive(Default)]
struct Prefs {
    wanted: Option<FullScreenPrefs>,
    sent: Option<FullScreenPrefs>,
}

/// What the minimise targets are worked out from, and what was last sent.
#[derive(Default)]
struct Targets {
    rects: OutputRects,
    sent: MinimiseTargets,
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
    pub fn absent(desktop: watch::Receiver<DesktopView>) -> ShellSeat {
        ShellSeat {
            link: None,
            arm: None,
            desktop,
            targets: Targets::default(),
            prefs: Prefs::default(),
        }
    }

    /// The host is on the connection the compositor handed over: the arm starts at the first
    /// mapped surface that has a role, and its notes reach `host`, which is also told each time
    /// `desktop` changes.
    pub fn linked(
        runtime: Handle,
        host: HostHandle<ShellMsg>,
        rects: watch::Receiver<OutputRects>,
        desktop: watch::Receiver<DesktopView>,
    ) -> ShellSeat {
        forward(&runtime, host.clone(), rects, ShellMsg::DockRects);
        nudge(&runtime, host.clone(), desktop.clone(), || {
            ShellMsg::Desktop
        });
        ShellSeat {
            link: Some(Link { runtime, host }),
            arm: None,
            desktop,
            targets: Targets::default(),
            prefs: Prefs::default(),
        }
    }

    /// Carry out `command` on the arm; false when there is none running (the commands of a
    /// session without the desktop's compositor have nowhere to go).
    pub fn command(&mut self, command: DesktopCommand) -> bool {
        let Some(arm) = &self.arm else {
            return false;
        };
        let sent = arm.command(command).is_ok();
        if !sent {
            eprintln!("sill: desktop: the desktop arm has stopped");
            self.arm = None;
        }
        sent
    }

    /// The desktop's windows as the windows service last published them. Read and let go: the
    /// publisher waits for the borrow.
    pub fn view(&self) -> watch::Ref<'_, DesktopView> {
        self.desktop.borrow()
    }

    /// The docks' icons rest somewhere new.
    pub fn set_rects(&mut self, rects: OutputRects) {
        self.targets.rects = rects;
        self.retarget();
    }

    /// The desktop's windows changed.
    pub fn desktop_changed(&mut self) {
        self.retarget();
    }

    /// The compositor is to hear `prefs` about full screen: sent now when they differ from what
    /// it last heard, and again when the arm starts.
    pub fn set_prefs(&mut self, prefs: FullScreenPrefs) {
        self.prefs.wanted = Some(prefs);
        self.send_prefs();
    }

    /// Send the full-screen prefs that differ from the ones last sent. Nothing without a running
    /// arm; a command that failed is sent again at the next change.
    fn send_prefs(&mut self) {
        let Some(wanted) = self.prefs.wanted else {
            return;
        };
        if self.arm.is_none() {
            return;
        }
        let commands = wanted.since(self.prefs.sent);
        let all_sent = commands.into_iter().all(|command| self.command(command));
        if all_sent {
            self.prefs.sent = Some(wanted);
        }
    }

    /// Send the minimise targets that differ from the ones last sent. Nothing without a
    /// running arm; a command that failed is sent again at the next change.
    fn retarget(&mut self) {
        if self.arm.is_none() {
            return;
        }
        let wanted = MinimiseTargets::of(&self.targets.rects, &self.desktop.borrow());
        let commands = wanted.since(&self.targets.sent);
        let all_sent = commands.into_iter().all(|command| self.command(command));
        if all_sent {
            self.targets.sent = wanted;
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
                    // A new arm is a new binding: it has heard nothing yet.
                    self.prefs.sent = None;
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
            return;
        }
        // A surface registered: the compositor may now accept targets it refused before.
        self.targets.sent = MinimiseTargets::default();
        self.retarget();
        self.send_prefs();
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
    let Some((output, role)) = surface_role(outputs, *id) else {
        return;
    };
    let Some(surface) = ctx.wl_surface(*id) else {
        return;
    };
    seat.mapped(role, &output, surface);
}

/// The output's name and the `Surface::ROLE` of the surface `id`, when it has one.
fn surface_role(outputs: &Outputs, id: SurfaceId) -> Option<(String, ShellRole)> {
    outputs.outputs.iter().find_map(|(info, surfaces)| {
        let role = surfaces.spawned_by(id)?.role?;
        Some((info.name.clone().unwrap_or_default(), role))
    })
}

/// Carry out one note of the compositor's shell protocol on the host thread.
pub fn apply(shell: &Shell, note: ShellNote) {
    match note {
        ShellNote::HotCorner(corner) => hot_corner(shell, corner),
        ShellNote::Gesture(gesture) => key_gesture(shell, gesture),
        ShellNote::Killed => companion_stop(shell),
        ShellNote::Modifiers(mods) => {
            // The hot corners pick their binding by these.
            shell.held_mods.send_replace(mods);
            option_variants();
        }
        ShellNote::Switcher(_) => switcher_panel(),
        bar @ (ShellNote::Overview(_)
        | ShellNote::OverviewOutput { .. }
        | ShellNote::DropTarget { .. }
        | ShellNote::DropTargetNew { .. }
        | ShellNote::WindowDragBegin { .. }
        | ShellNote::WindowDragEnd { .. }
        | ShellNote::SpacePicture { .. }) => {
            crate::spaces_bar::apply_note(&shell.links.spaces_bar, &bar);
        }
        picture @ ShellNote::OutputPicture { .. } => crate::eyedropper::apply_note(
            &shell.links.eyedropper,
            &shell.services.capture.control,
            picture,
        ),
        ShellNote::Reveal {
            output,
            edge,
            shown,
        } => edge_reveal(shell, output.as_deref(), edge, shown),
        ShellNote::MenuFlash { .. } => menu_flash(),
        ShellNote::Bell(_) => bell(),
        ShellNote::Reopen(_) => reopen_in_dock(),
        ShellNote::WindowMenu {
            window,
            at,
            enabled,
        } => shell.links.window_menu.open(window, at, enabled),
        ShellNote::SplitChoose { .. }
        | ShellNote::SplitCandidate(_)
        | ShellNote::SplitChoiceEnd(_) => {
            crate::split_picker::apply(&shell.links.split_picker, &shell.seat.view(), note);
        }
        ShellNote::PermissionAsk(consent) => {
            let _capture_may_be_gone = shell
                .services
                .capture
                .control
                .send(CaptureCommand::Consent(consent));
        }
        trial @ (ShellNote::DisplaysTrial(_) | ShellNote::DisplaysTrialEnd(_)) => {
            crate::display_trial::apply(&shell.links.display_trial, trial);
        }
        ShellNote::PermissionWithdraw(ask) => {
            let _capture_may_be_gone = shell
                .services
                .capture
                .control
                .send(CaptureCommand::ConsentWithdrawn(ask));
        }
    }
}

/// The pointer pushed into a corner: the corner link's firing, which `corners::spawn_hits`
/// carries out as the corner's configured action. The compositor names no output, so the main
/// display's is used.
fn hot_corner(shell: &Shell, corner: ProtocolCorner) {
    let corner = match corner {
        ProtocolCorner::TopLeft => Corner::TopLeft,
        ProtocolCorner::TopRight => Corner::TopRight,
        ProtocolCorner::BottomLeft => Corner::BottomLeft,
        ProtocolCorner::BottomRight => Corner::BottomRight,
    };
    if let Some(output) = shell.outputs.pick(OutputPolicy::Main, None) {
        shell.links.corners.hit(corner, output);
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

/// The modifiers over a shell surface will switch the dock's and bar's Option variants (they
/// already pick a hot corner's binding, `Shell::held_mods`); keycap stays the source of truth for chords. Lane P3-SILL.
fn option_variants() {}

/// The compositor's switcher events (apps, shown, selected, hidden) will drive the switcher
/// panel, answered with `PickSwitcher` and `CancelSwitcher`; the panel still follows its own
/// machine. Lane P3-SW (casement side) and P3-SILL (the panel).
fn switcher_panel() {}

/// An edge reveal shows or hides the bar (top edge) or the dock (bottom edge) while a window is
/// full screen. The note names no output: a reveal is for the focused window's output (the
/// full-screen one), a hide for every output.
fn edge_reveal(shell: &Shell, output: Option<&str>, edge: ScreenEdge, shown: Shown) {
    let focus = shell
        .services
        .windows
        .state
        .borrow()
        .toplevels
        .focused_output();
    let focused = shell
        .outputs
        .pick(OutputPolicy::FocusedWindow, focus.as_ref());
    let named = output.map(|name| shell.outputs.named(name));
    let landing = crate::edge_reveal::landing(named, shown, focused);
    crate::edge_reveal::apply(&shell.links.edges, edge, shown, landing);
}

/// A menu item activated by a key will flash in the bar's menu. Lane P3-MN.
fn menu_flash() {}

/// An app's bell will bounce its dock icon or play the alert sound. Lane P3-SILL.
fn bell() {}

/// The compositor asks the dock to reopen an app with no windows; what reopening means (launch
/// it again, or bring its last window back) is the dock's decision. Lane P3-SILL.
fn reopen_in_dock() {}
