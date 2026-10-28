//! The shell's actions, carried out: the one interpreter. Every entrance (a menu, a corner, a
//! launcher row, the command line, an agent) hands it an [`Ask`] and where it came from; it owns
//! the three decisions an entrance used to make for itself: which output a panel opens on
//! ([`Outputs::open_on`](crate::outputs::Outputs::open_on), unless the origin names one), the
//! activation token of a launch (asked of the origin's surface, always), and whether a power action
//! asks first or happens at once (`SessionAction::route`). What an action is, is
//! `sill_model::actions`'s; this is where its `Effect` is matched.
//!
//! It runs on the host thread, which holds the surfaces a token is asked of. A surface's ask
//! arrives on the `ActionLink` and is forwarded here by [`spawn_asks`]; a client's arrives as
//! `Request::Do`. Nothing here goes over D-Bus.

mod app;
pub(crate) mod input_source;
mod launch;
pub mod next_window;
pub(crate) mod panel;
mod pref;
pub mod show_desktop;
pub mod switcher;
pub mod workspace;

use shell_host::{HostCtx, HostHandle};
use sill_ipc::response::{FailureKind, Response};
use sill_ipc::show::Reveal;
use sill_launcher::{AppId, Route, Scope, SessionAction};
use sill_model::actions::{Ask, DesktopStep, Effect, Subject, def};
use sill_model::launch::{LaunchCommand, LaunchRequest, SETTINGS_BUS_NAME, SettingsOpen};
use sill_model::locker::LockerCommand;
use sill_model::screenshots::{ShotCommand, ShotKind};
use sill_model::session::SessionCommand;
use sill_ui_kit::links::{ActionLink, Asked, Origin};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;
use crate::ipc::dispatch::{Reach, failed, sent};

/// Do what `ask` says, from `origin`.
pub fn perform(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, ask: Ask, origin: Origin) -> Response {
    if let Some(why) = ask.refusal() {
        return failed(FailureKind::Refused, why);
    }
    let Ask { action, subject } = ask;
    match def(action).effect {
        Effect::Session(action) => session(reach, action, origin),
        Effect::Panel(panel) => panel::show(reach, panel, reveal_of(panel.opens()), origin.output),
        Effect::Find(kind) => panel::launcher_in(reach, Scope::Only(kind), origin.output),
        Effect::Switcher(step) => switcher::chord(reach, step),
        Effect::Shot(kind) => screenshot(reach, kind),
        Effect::Desktop(step) => desktop(reach, step),
        Effect::Settings => settings(ctx, reach, subject, origin),
        Effect::App(verb) => app::perform(ctx, reach, verb, subject, origin),
        Effect::Pref(pref) => pref::flip(reach, pref),
        Effect::EmptyTrash => panel::empty_trash(reach, origin.output),
        Effect::InputSource => input_source::next(reach),
        Effect::Reload => reload(ctx, reach),
    }
}

/// How a panel action changes its panel, in the words of the wire: `opens` when it only opens it.
fn reveal_of(opens: bool) -> Reveal {
    match opens {
        false => Reveal::Toggle,
        true => Reveal::Open,
    }
}

/// A power action: asked about in the power menu, or at once. Lock is the locker's, which the
/// session service would only hand it to.
fn session(reach: &Reach, action: SessionAction, origin: Origin) -> Response {
    let services = reach.services;
    match action.route() {
        Route::Ask(action) => panel::ask_session(reach, action, origin.output),
        Route::Now(SessionAction::Lock) => {
            sent(services.locker.control.send(LockerCommand::Lock), "locker")
        }
        Route::Now(action) => sent(
            services.session.control.send(SessionCommand::Run(action)),
            "session",
        ),
    }
}

fn screenshot(reach: &Reach, kind: ShotKind) -> Response {
    sent(
        reach
            .services
            .screenshots
            .control
            .send(ShotCommand::Take(kind)),
        "screenshots",
    )
}

fn desktop(reach: &Reach, step: DesktopStep) -> Response {
    match step {
        DesktopStep::ShowDesktop => show_desktop::toggle(reach),
        DesktopStep::MissionControl => workspace::overview(reach),
        DesktopStep::Space(step) => workspace::step(reach, step),
        DesktopStep::NextWindow => next_window::ask(reach),
    }
}

/// Settings, at the place `subject` names or where it was; the window it opens is let to the
/// front with the origin's token.
fn settings(
    ctx: &mut HostCtx<ShellMsg>,
    reach: &Reach,
    subject: Subject,
    origin: Origin,
) -> Response {
    let launch = reach.services.launch.control.clone();
    launch::with_token(
        ctx,
        reach,
        origin,
        SETTINGS_BUS_NAME,
        move |token| match subject {
            Subject::Settings(link) => launch.send(LaunchCommand::OpenSettings(
                SettingsOpen::at(link).with_token(token),
            )),
            Subject::None | Subject::App(_) | Subject::Start(_) | Subject::Windows(_) => launch
                .send(LaunchCommand::Launch(
                    LaunchRequest::app(AppId(SETTINGS_BUS_NAME.to_owned())).token(token),
                )),
        },
    )
}

/// Re-read `settings.toml` and both `style.css` files and publish them: the bar and every surface pick them
/// up through the settings feed, the scroll engine through `ctx`. Dock pins are not re-read here: the pins
/// service watches its own file.
pub fn reload(ctx: &mut HostCtx<ShellMsg>, reach: &Reach) -> Response {
    crate::daemon::settings::reload(reach.store, reach.settings);
    reach.styles.reload();
    ctx.set_scroll_config(crate::scroll::config(&reach.settings.borrow()));
    Response::Done
}

/// Follow `link` on `rt` and hand each ask to the host thread, which carries it out
/// ([`perform`]), until the host is gone.
pub fn spawn_asks(rt: &Handle, link: &ActionLink, host: HostHandle<ShellMsg>) {
    let mut asks = link.subscribe();
    rt.spawn(async move {
        loop {
            if asks.changed().await.is_err() {
                return;
            }
            let Some(latest) = asks.borrow_and_update().clone() else {
                continue;
            };
            if host.send(ShellMsg::Action(latest.item)).is_err() {
                return;
            }
        }
    });
}

/// What the host says of an ask that failed: nobody is waiting for the answer, so it is logged.
pub fn report(asked: &Asked, response: &Response) {
    if let Response::Failed(failure) = response {
        eprintln!("sill: {}: {}", def(asked.ask.action).label, failure.message);
    }
}
