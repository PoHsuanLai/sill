//! `org.quire.Displays1` on a private bus: the levels are listed to anyone, only Settings sets
//! one (a stranger is refused and nothing reaches the displays service), a change of the
//! displays is announced. The displays are a made-up state; no logind, no DDC, no real bus. A
//! test skips, saying so, without a `dbus-daemon`.

use std::pin::Pin;
use std::time::Duration;

use sill_model::displays::brightness::{Backlight, BacklightKind, BacklightName};
use sill_model::displays::ddc::{Controls, DdcDisplay, Endpoint, Level};
use sill_model::displays::output::describe;
use sill_model::displays::randr::Head;
use sill_model::displays::{Displays, DisplaysCommand};
use sill_model::ids::{DdcId, OutputName};
use sill_model::serve::ServeLink;
use sill_model::units::Percent;
use sill_services::control::Control;
use sill_services::displays::bus::{BUS, DisplaysBusLinks, DisplaysBusService, PATH};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::{Handle, spawn};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::export::futures_core::Stream;

use crate::accounts_sheet_bus::connect;
use crate::callers_support::{Introduced, Role, Who};

const SETTLE: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_millis(500);
const INTERFACE: &str = "org.quire.Displays1";

struct Shell {
    service: Handle<DisplaysBusService>,
    callers: std::sync::Arc<Introduced>,
    displays: watch::Sender<Displays>,
    commands: mpsc::UnboundedReceiver<DisplaysCommand>,
    _scratch: TempDir,
}

fn desk(panel_value: u32) -> Displays {
    let head = |n: &str| Head {
        name: OutputName(n.to_owned()),
        ..Head::default()
    };
    Displays {
        outputs: ["eDP-1", "HDMI-A-2", "DP-1"]
            .iter()
            .map(|n| describe(&head(n), None))
            .collect(),
        ddc: vec![DdcDisplay {
            endpoint: Endpoint {
                id: DdcId(5),
                connector: Some(OutputName("HDMI-A-2".to_owned())),
                product: "Monitor".to_owned(),
            },
            controls: Controls {
                brightness: Some(Level {
                    value: 49,
                    max: 100,
                }),
                ..Controls::default()
            },
        }],
        backlights: vec![Backlight {
            name: BacklightName("intel_backlight".to_owned()),
            kind: BacklightKind::Raw,
            value: panel_value,
            max: 100,
        }],
    }
}

fn shell(bus: &PrivateBus) -> Shell {
    let scratch = TempDir::new("displays-bus");
    let callers = Introduced::new();
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        callers: callers.clone(),
        claims: sill_services::env::NameClaims::Claim,
        ..Env::isolated(scratch.path())
    };
    let (displays, watched) = watch::channel(desk(50));
    let (control, commands) = Control::channel();
    let service = spawn::<DisplaysBusService>(
        &tokio::runtime::Handle::current(),
        &env,
        DisplaysBusLinks {
            displays: watched,
            control,
        },
    );
    Shell {
        service,
        callers,
        displays,
        commands,
        _scratch: scratch,
    }
}

async fn serving(shell: &mut Shell) {
    let seen = tokio::time::timeout(
        SETTLE,
        shell.service.state.wait_for(|s| *s == ServeLink::Serving),
    )
    .await;
    assert!(seen.is_ok_and(|s| s.is_ok()), "the name was not claimed");
}

async fn standing_in(bus: &PrivateBus, shell: &Shell, name: &str, who: Who) -> Connection {
    let connection = connect(bus, Some(name)).await;
    shell.callers.make(&connection, who);
    connection
}

async fn settings_of(bus: &PrivateBus, shell: &Shell) -> Connection {
    let who = Who::Service(Role::Settings);
    standing_in(bus, shell, "org.quire.Settings", who).await
}

async fn levels(connection: &Connection) -> Vec<(String, u32)> {
    connection
        .call_method(Some(BUS), PATH, Some(INTERFACE), "Brightness", &())
        .await
        .unwrap_or_else(|e| panic!("{e}"))
        .body()
        .deserialize()
        .unwrap_or_else(|e| panic!("{e}"))
}

async fn set(connection: &Connection, output: &str, percent: u32) -> zbus::Result<()> {
    connection
        .call_method(
            Some(BUS),
            PATH,
            Some(INTERFACE),
            "SetBrightness",
            &(output, percent),
        )
        .await
        .map(|_| ())
}

async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

fn pair(name: &str, percent: u32) -> (String, u32) {
    (name.to_owned(), percent)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_levels_are_listed_and_settings_sets_one() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let settings = settings_of(&bus, &shell).await;
    serving(&mut shell).await;
    assert_eq!(
        levels(&settings).await,
        vec![pair("eDP-1", 50), pair("HDMI-A-2", 49)]
    );
    set(&settings, "HDMI-A-2", 30)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let heard = tokio::time::timeout(SETTLE, shell.commands.recv()).await;
    assert_eq!(
        heard.ok().flatten(),
        Some(DisplaysCommand::SetBrightness {
            output: OutputName("HDMI-A-2".to_owned()),
            level: Percent(30),
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_or_a_bad_request_changes_nothing() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let settings = settings_of(&bus, &shell).await;
    let stranger = standing_in(&bus, &shell, "org.example.Stranger", Who::Program).await;
    serving(&mut shell).await;
    assert!(set(&stranger, "eDP-1", 10).await.is_err());
    assert!(set(&settings, "DP-1", 10).await.is_err(), "unlisted output");
    assert!(set(&settings, "eDP-1", 101).await.is_err(), "over 100");
    assert_eq!(levels(&stranger).await.len(), 2, "reading is open");
    let nothing = tokio::time::timeout(QUIET, shell.commands.recv()).await;
    assert!(nothing.is_err(), "a refused call reached the displays");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_of_the_levels_is_announced() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let settings = settings_of(&bus, &shell).await;
    serving(&mut shell).await;
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface(INTERFACE)
        .and_then(|b| b.member("Changed"))
        .map(|b| b.build())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut changes = zbus::MessageStream::for_match_rule(rule, &settings, None)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let _ = shell.displays.send(desk(80));
    let message = tokio::time::timeout(SETTLE, next(&mut changes))
        .await
        .unwrap_or_else(|_| panic!("no Changed"))
        .unwrap_or_else(|| panic!("the stream ended"))
        .unwrap_or_else(|e| panic!("{e}"));
    let heard: Vec<(String, u32)> = message
        .body()
        .deserialize()
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(heard, vec![pair("eDP-1", 80), pair("HDMI-A-2", 49)]);
}
