//! The screen-sharing picker's host side on a private bus: a stand-in transom (the owner of the
//! portal backend's name) calls `PickCaptureSource` on the connection the seat holds; the shell's
//! own part is stepped through `Answer`. Only the portal backend may ask (who counts as it is
//! `callers_bus.rs`'s), and a second call while one is open is refused. `PickColor` is the same:
//! one question at a time, an empty array while nothing draws the eyedropper, the colour once it
//! is clicked. A test skips, saying so, without a `dbus-daemon`. Never the person's session bus.

use std::time::Duration;

use sill_model::apps::AppIndex;
use sill_model::capture_pick::{
    CaptureCommand, CaptureLink, CapturePicker, ColorOutcome, Eyedropper, Outcome, Picks,
    Plurality, Rgb8, Source, WindowCandidate, plan, toggle,
};
use sill_model::desktop::DesktopCommand;
use sill_model::displays::Displays;
use sill_services::capture_pick::{CaptureLinks, CaptureService};
use sill_services::env::Env;
use sill_services::seat::seat;
use sill_services::service::spawn;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::Connection;
use zbus::connection::Builder;

use crate::callers_support::{Introduced, Role, Who};

const SETTLE: Duration = Duration::from_secs(5);
const PORTAL: &str = "org.freedesktop.impl.portal.desktop.quire";

#[zbus::proxy(
    interface = "org.quire.Shell1",
    default_service = "org.quire.Shell",
    default_path = "/org/quire/Shell1",
    gen_blocking = false
)]
pub(crate) trait Shell1 {
    fn pick_capture_source(
        &self,
        app_label: &str,
        kinds: u32,
        multiple: u32,
        cursor_modes: Vec<u32>,
    ) -> zbus::Result<(Vec<(String, String)>, u32)>;

    fn pick_color(&self, app_label: &str) -> zbus::Result<Vec<(f64, f64, f64)>>;

    fn ask_permission(
        &self,
        app_label: &str,
        title: &str,
        subtitle: &str,
        body: &str,
        grant_label: &str,
        deny_label: &str,
    ) -> zbus::Result<u32>;
}

pub(crate) async fn wait_for<T>(
    receiver: &mut watch::Receiver<T>,
    want: impl Fn(&T) -> bool,
) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

pub(crate) async fn connect(bus: &PrivateBus, name: Option<&str>) -> Connection {
    let builder = Builder::address(bus.address.as_str()).unwrap_or_else(|e| panic!("{e}"));
    let builder = match name {
        Some(name) => builder
            .name(name.to_owned())
            .unwrap_or_else(|e| panic!("{e}")),
        None => builder,
    };
    builder
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting: {e}"))
}

fn notes() -> WindowCandidate {
    WindowCandidate {
        identifier: "w-1".to_owned(),
        title: "Notes".to_owned(),
        app: "Notes".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn transom_asks_the_person_picks_and_the_answer_comes_back() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-bus");
    let (mut service, transom) = serving(&bus, &scratch).await;
    let _ = service.control.send(CaptureCommand::Windows(vec![notes()]));

    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let first = {
        let proxy = proxy.clone();
        tokio::spawn(async move { proxy.pick_capture_source("Meet", 2, 0, vec![1, 2]).await })
    };
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.asking.is_some()).await,
        "the sheet was not opened"
    );
    // One at a time: the second call is refused at once.
    assert!(
        proxy
            .pick_capture_source("Other", 1, 0, vec![1])
            .await
            .is_err()
    );

    let (id, ask, candidates) = {
        let state = service.state.borrow();
        let asking = state.asking.clone().unwrap_or_else(|| panic!("open"));
        (asking.id, asking.ask, state.candidates.clone())
    };
    let plan = plan(&ask, &candidates);
    let picks = toggle(
        &plan,
        Plurality::One,
        Picks::start(&plan),
        Source::Window("w-1".to_owned()),
    );
    let _ = service
        .control
        .send(CaptureCommand::Answer(id, Outcome::Allow(picks)));
    let answer = first
        .await
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, (vec![("window".to_owned(), "w-1".to_owned())], 1));
}

/// The service serving on a private bus, with a stand-in transom connected.
pub(crate) async fn serving(
    bus: &PrivateBus,
    scratch: &TempDir,
) -> (sill_services::service::Handle<CaptureService>, Connection) {
    let rig = rig(bus, scratch).await;
    (rig.service, rig.transom)
}

/// The service serving on a private bus: what stands in for transom, for the shell's seat, and
/// for the compositor (the answers it hears).
pub(crate) struct Rig {
    pub service: sill_services::service::Handle<CaptureService>,
    /// The portal backend: the owner of its name, introduced as the program that is.
    pub transom: Connection,
    /// What the compositor is told the person answered.
    pub answers: tokio::sync::mpsc::UnboundedReceiver<DesktopCommand>,
    /// Who the stand-ins are, to the shell.
    pub callers: std::sync::Arc<Introduced>,
}

pub(crate) async fn rig(bus: &PrivateBus, scratch: &TempDir) -> Rig {
    rig_as(bus, scratch, Who::Service(Role::Portal)).await
}

/// As [`rig`], the owner of the portal backend's name being `who` to the shell.
pub(crate) async fn rig_as(bus: &PrivateBus, scratch: &TempDir, who: Who) -> Rig {
    let callers = Introduced::new();
    let env = Env {
        callers: callers.clone(),
        ..Env::isolated(scratch.path())
    };
    let transom = connect(bus, Some(PORTAL)).await;
    callers.make(&transom, who);
    let shell = connect(bus, Some("org.quire.Shell")).await;
    let (publisher, view) = seat();
    publisher.publish(shell);
    let (_displays_tx, displays) = watch::channel(Displays::default());
    let (answers_tx, answers) = tokio::sync::mpsc::unbounded_channel();
    let mut service = spawn::<CaptureService>(
        &tokio::runtime::Handle::current(),
        &env,
        CaptureLinks {
            seat: view,
            displays,
            apps: watch::channel(AppIndex::default()).1,
            answers: answers_tx,
        },
    );
    let _ = service.control.send(CaptureCommand::Serve);
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.link
            == CaptureLink::Serving)
        .await
    );
    // The object is exported a moment after the link says serving.
    tokio::time::sleep(Duration::from_millis(300)).await;
    Rig {
        service,
        transom,
        answers,
        callers,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_colour_request_is_cancelled_at_once_while_nothing_draws_the_eyedropper() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-colour-absent");
    let (service, transom) = serving(&bus, &scratch).await;
    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let answer = proxy
        .pick_color("Screenshot")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, Vec::<(f64, f64, f64)>::new());
    assert!(service.state.borrow().color.is_none());
}

/// Tell the service a surface draws the eyedropper and wait until it has taken that in. The
/// service reads its commands and the bus calls on two channels and takes whichever is ready, so
/// a call made straight after the command can be handled first and cancelled at once.
async fn draw_eyedropper(service: &mut sill_services::service::Handle<CaptureService>) {
    let _ = service
        .control
        .send(CaptureCommand::Eyedropper(Eyedropper::Present));
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.eyedropper
            == Eyedropper::Present)
        .await,
        "the eyedropper was not taken in"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_colour_is_asked_one_at_a_time_and_the_click_comes_back_as_floats() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-colour");
    let (mut service, transom) = serving(&bus, &scratch).await;
    draw_eyedropper(&mut service).await;
    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let first = {
        let proxy = proxy.clone();
        tokio::spawn(async move { proxy.pick_color("Screenshot").await })
    };
    assert!(
        wait_for(&mut service.state, |s: &CapturePicker| s.color.is_some()).await,
        "the eyedropper was not opened"
    );
    // One at a time, of either kind: both second calls are refused at once.
    assert!(proxy.pick_color("Other").await.is_err());
    assert!(
        proxy
            .pick_capture_source("Other", 1, 0, vec![1])
            .await
            .is_err()
    );
    let id = service
        .state
        .borrow()
        .color
        .as_ref()
        .map(|asking| asking.id)
        .unwrap_or_else(|| panic!("open"));
    let _ = service.control.send(CaptureCommand::ColorAnswer(
        id,
        ColorOutcome::Picked(Rgb8 {
            red: 255,
            green: 0,
            blue: 51,
        }),
    ));
    let answer = first
        .await
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, vec![(1.0, 0.0, 51.0 / 255.0)]);
    assert!(
        service.state.borrow().color.is_none(),
        "the eyedropper is free again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_colour_request_from_anyone_but_the_portal_backend_is_refused() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-colour-forged");
    let (mut service, _transom) = serving(&bus, &scratch).await;
    draw_eyedropper(&mut service).await;
    let forger = connect(&bus, None).await;
    let proxy = Shell1Proxy::new(&forger)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(proxy.pick_color("Evil").await.is_err());
    assert!(service.state.borrow().color.is_none());
}
