//! The keycap service against a fake daemon: a `SOCK_SEQPACKET` socket in a scratch folder
//! stands in for `/run/keycap/keycap.sock`. No system bus, no real daemon, no real config folder.

use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use keycap_protocol::{
    Grab, Keyboard, KeyboardInfo, Mode, PROTOCOL_VERSION, ProfileName, Status, ToClient, ToDaemon,
    decode, encode,
};
use rustix::event::{PollFd, PollFlags, poll};
use rustix::net::{
    AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType, accept, bind,
    listen, recv, send, socket_with,
};
use rustix::time::Timespec;
use sill_launcher::{AppId, WindowKey};
use sill_model::apps::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal, step as apps_step,
};
use sill_model::cosmic_wl::{Activity, CosmicWl, Sizing, Toplevel, Toplevels, WindowState};
use sill_model::keyboard::UserOverrides;
use sill_model::keycap::{Attention, KeycapCommand, KeycapState, notice_text};
use sill_model::notifications::{
    BannerLife, LiveBanner, NotesState, Notification, NotificationId, NotificationsCommand, UnixMs,
};
use sill_services::Control;
use sill_services::env::Env;
use sill_services::keycap::{KeycapLinks, KeycapService};
use sill_services::service::spawn;
use sill_testkit::TempDir;
use tokio::sync::{mpsc, watch};

const SHORT: Duration = Duration::from_millis(300);
const LONG: Duration = Duration::from_secs(5);

fn name(s: &str) -> ProfileName {
    ProfileName::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

fn focus(s: &str) -> ToDaemon {
    ToDaemon::Focus { profile: name(s) }
}

fn app(id: &str, categories: &[&str]) -> App {
    App {
        id: AppId(id.to_owned()),
        name: id.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: id.to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: Vec::new(),
        mime_types: Vec::new(),
        categories: categories.iter().map(|c| (*c).to_owned()).collect(),
        keyboard_profile: None,
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    }
}

fn apps() -> AppIndex {
    apps_step(
        AppIndex::default(),
        AppsEvent::Scanned(vec![
            app("foot", &["TerminalEmulator"]),
            app("kitty", &["TerminalEmulator"]),
            app("plain", &["Utility"]),
        ]),
    )
}

/// The compositor's state with `app_id` active, or no window at all.
fn focused(app_id: Option<&str>) -> CosmicWl {
    let windows = app_id
        .into_iter()
        .map(|app_id| Toplevel {
            key: WindowKey(1),
            app_id: app_id.to_owned(),
            title: String::new(),
            activity: Activity::Active,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: Vec::new(),
        })
        .collect();
    CosmicWl {
        toplevels: Toplevels { windows },
        ..CosmicWl::default()
    }
}

fn status(mode: Mode, grabs: &[(&str, Grab)]) -> Status {
    Status {
        mode,
        keyboards: grabs
            .iter()
            .map(|&(device, grab)| Keyboard {
                info: KeyboardInfo {
                    name: device.to_owned(),
                    path: format!("/dev/input/{device}"),
                    vendor: 0x5ac,
                    product: 0x250,
                },
                grab,
            })
            .collect(),
    }
}

fn running() -> Status {
    status(Mode::Running, &[("k1", Grab::Active)])
}

/// Wait up to `limit` for `fd` to be readable; whether it is.
fn readable(fd: &OwnedFd, limit: Duration) -> bool {
    let mut fds = [PollFd::new(fd, PollFlags::IN | PollFlags::HUP)];
    let timeout = Timespec {
        tv_sec: i64::try_from(limit.as_secs()).unwrap_or(i64::MAX),
        tv_nsec: i64::from(limit.subsec_nanos()),
    };
    poll(&mut fds, Some(&timeout)).is_ok_and(|ready| ready > 0)
}

/// The fake keycap: a listening socket.
struct Daemon {
    listener: OwnedFd,
    _folder: TempDir,
}

impl Daemon {
    fn start() -> (Daemon, PathBuf) {
        let folder = TempDir::new("keycap-daemon");
        let path = folder.path().join("keycap.sock");
        let listener = socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap_or_else(|e| panic!("socket: {e}"));
        bind(
            &listener,
            &SocketAddrUnix::new(&path).unwrap_or_else(|e| panic!("{e}")),
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        listen(&listener, 4).unwrap_or_else(|e| panic!("listen: {e}"));
        (
            Daemon {
                listener,
                _folder: folder,
            },
            path,
        )
    }

    /// The next client to connect.
    fn accept(&self) -> Peer {
        assert!(readable(&self.listener, LONG), "no client connected");
        Peer(accept(&self.listener).unwrap_or_else(|e| panic!("accept: {e}")))
    }
}

/// One client, as the daemon sees it.
struct Peer(OwnedFd);

impl Peer {
    fn tell(&self, message: &ToClient) {
        send(&self.0, &encode(message), SendFlags::NOSIGNAL)
            .unwrap_or_else(|e| panic!("send: {e}"));
    }

    /// The next message from the client; `None` when nothing came within `limit` or it hung up.
    fn next(&self, limit: Duration) -> Option<ToDaemon> {
        if !readable(&self.0, limit) {
            return None;
        }
        let mut buffer = vec![0u8; keycap_protocol::MAX_PACKET];
        let (len, _) = recv(&self.0, &mut buffer[..], RecvFlags::empty()).ok()?;
        (len > 0).then(|| decode(&buffer[..len]).unwrap_or_else(|e| panic!("{e}")))
    }
}

/// The service, the fake daemon and the other services' ends.
struct Rig {
    daemon: Daemon,
    state: watch::Receiver<KeycapState>,
    control: Control<KeycapCommand>,
    wl: watch::Sender<CosmicWl>,
    /// Held so the index stays readable; these tests never change it.
    _apps: watch::Sender<AppIndex>,
    notices: mpsc::UnboundedReceiver<NotificationsCommand>,
    notes: watch::Sender<NotesState>,
    scratch: TempDir,
}

impl Rig {
    /// Where the service looks for `keycap.toml`.
    fn config_dir(&self) -> PathBuf {
        sill_services::env::Dirs::under(self.scratch.path()).keycap_config()
    }
}

/// The service against a fake daemon, with `config` as the person's `keycap.toml` if given.
fn rig(config: Option<&str>) -> Rig {
    let (daemon, socket) = Daemon::start();
    let scratch = TempDir::new("keycap-scratch");
    let config_dir = sill_services::env::Dirs::under(scratch.path()).keycap_config();
    if let Some(text) = config {
        std::fs::create_dir_all(&config_dir).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(config_dir.join("keycap.toml"), text).unwrap_or_else(|e| panic!("{e}"));
    }
    let env = Env {
        keycap_socket: Some(socket),
        ..Env::isolated(scratch.path())
    };
    let (apps_tx, apps_rx) = watch::channel(apps());
    let (wl, wl_rx) = watch::channel(focused(None));
    let (notifications, notices) = Control::channel();
    let (notes, notes_rx) = watch::channel(NotesState::default());
    let links = KeycapLinks {
        cosmic_wl: wl_rx,
        apps: apps_rx,
        notifications,
        notes: notes_rx,
    };
    let handle = spawn::<KeycapService>(&tokio::runtime::Handle::current(), &env, links);
    Rig {
        daemon,
        state: handle.state,
        control: handle.control,
        wl,
        _apps: apps_tx,
        notices,
        notes,
        scratch,
    }
}

async fn until(state: &mut watch::Receiver<KeycapState>, want: impl Fn(&KeycapState) -> bool) {
    let give_up = Instant::now() + LONG;
    loop {
        let now = state.borrow_and_update().clone();
        if want(&now) {
            return;
        }
        let left = give_up.saturating_duration_since(Instant::now());
        if tokio::time::timeout(left, state.changed()).await.is_err() {
            panic!("gave up at {now:?}");
        }
    }
}

/// A connected client that has been greeted: `Hello` sent, `Connected` seen.
async fn connected(rig: &mut Rig) -> Peer {
    let peer = rig.daemon.accept();
    peer.tell(&ToClient::Hello {
        version: PROTOCOL_VERSION,
        status: running(),
    });
    until(&mut rig.state, |s| {
        *s == KeycapState::Connected { status: running() }
    })
    .await;
    peer
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hello_connects_and_the_current_focus_is_told() {
    let mut rig = rig(None);
    let peer = connected(&mut rig).await;
    assert_eq!(peer.next(LONG), Some(focus("default")));
    assert_eq!(peer.next(SHORT), None, "no config file, nothing more");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_focus_change_to_a_terminal_is_told_once_per_profile_change() {
    let mut rig = rig(None);
    let peer = connected(&mut rig).await;
    assert_eq!(peer.next(LONG), Some(focus("default")));

    rig.wl.send_replace(focused(Some("foot")));
    assert_eq!(peer.next(LONG), Some(focus("terminal")));

    rig.wl.send_replace(focused(Some("kitty")));
    assert_eq!(peer.next(SHORT), None, "another terminal, the same profile");

    rig.wl.send_replace(focused(Some("plain")));
    assert_eq!(
        peer.next(LONG),
        Some(focus("default")),
        "the next thing keycap hears is the change back"
    );
}

fn overriding(rows: &[(&str, &str)]) -> KeycapCommand {
    KeycapCommand::Overrides(UserOverrides(
        rows.iter()
            .map(|&(app, profile)| (AppId(app.to_owned()), name(profile)))
            .collect(),
    ))
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_for_the_focused_app_is_told_and_each_change_is_told_again() {
    let mut rig = rig(None);
    let peer = connected(&mut rig).await;
    assert_eq!(peer.next(LONG), Some(focus("default")), "no window yet");
    rig.wl.send_replace(focused(Some("foot")));
    assert_eq!(
        peer.next(LONG),
        Some(focus("terminal")),
        "the category, no override"
    );

    let send = |command| {
        rig.control
            .send(command)
            .unwrap_or_else(|_| panic!("the service is gone"))
    };
    send(overriding(&[("foot", "writing")]));
    assert_eq!(
        peer.next(LONG),
        Some(focus("writing")),
        "an override beats the category"
    );

    send(overriding(&[("foot", "code")]));
    assert_eq!(peer.next(LONG), Some(focus("code")), "a changed override");

    send(overriding(&[("foot", "code"), ("plain", "writing")]));
    assert_eq!(
        peer.next(SHORT),
        None,
        "an override for another app changes nothing"
    );

    send(overriding(&[]));
    assert_eq!(
        peer.next(LONG),
        Some(focus("terminal")),
        "no override: the category again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_server_closing_is_a_restart_and_the_next_connection_is_told_the_focus_again() {
    let mut rig = rig(None);
    let peer = connected(&mut rig).await;
    assert_eq!(peer.next(LONG), Some(focus("default")));
    rig.wl.send_replace(focused(Some("foot")));
    assert_eq!(peer.next(LONG), Some(focus("terminal")));

    drop(peer);
    until(&mut rig.state, |s| *s == KeycapState::Restarting).await;

    let again = connected(&mut rig).await;
    assert_eq!(
        again.next(LONG),
        Some(focus("terminal")),
        "a new connection starts from the focus as it is now"
    );
    assert_eq!(again.next(SHORT), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_config_file_is_told_on_connect_and_again_when_it_changes() {
    let mut rig = rig(Some("[profile.term]\n"));
    let peer = connected(&mut rig).await;
    let config = |toml: &str| ToDaemon::UserConfig {
        toml: toml.to_owned(),
    };
    assert_eq!(peer.next(LONG), Some(focus("default")));
    assert_eq!(peer.next(LONG), Some(config("[profile.term]\n")));
    assert_eq!(peer.next(SHORT), None);

    std::fs::write(rig.config_dir().join("keycap.toml"), "[profile.term2]\n")
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(peer.next(LONG), Some(config("[profile.term2]\n")));

    std::fs::remove_file(rig.config_dir().join("keycap.toml")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        peer.next(LONG),
        Some(config("")),
        "a deleted file is an empty config, so keycap forgets the old one"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_of_another_protocol_version_is_left_alone() {
    let rig = rig(None);
    let peer = rig.daemon.accept();
    peer.tell(&ToClient::Hello {
        version: PROTOCOL_VERSION + 1,
        status: running(),
    });
    assert_eq!(peer.next(SHORT), None, "nothing is sent to it");
    assert_eq!(*rig.state.borrow(), KeycapState::Unknown);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_notice_is_posted_replaced_and_withdrawn_as_keycap_needs() {
    let mut rig = rig(None);
    let peer = connected(&mut rig).await;
    tokio::time::sleep(SHORT).await;
    assert_eq!(
        rig.notices.try_recv().ok(),
        None,
        "a keyboard that works is no news"
    );

    peer.tell(&ToClient::Status(status(
        Mode::Running,
        &[("k1", Grab::Active), ("k2", Grab::Conflict)],
    )));
    let posted = next_notice(&mut rig.notices).await;
    let NotificationsCommand::Post(posting) = posted else {
        panic!("{posted:?}");
    };
    let conflict = Attention::Conflict {
        device: "k2".to_owned(),
    };
    assert_eq!(posting.summary, notice_text(&conflict).summary);
    assert_eq!(posting.body.text(), notice_text(&conflict).body);
    assert_eq!(posting.replaces, None);
    assert!(posting.actions.is_empty(), "no action can reach a service");

    // The notifications service now holds it as id 7.
    let held = Notification::posted(NotificationId(7), UnixMs(0), *posting);
    rig.notes.send_replace(NotesState {
        live: sill_model::notifications::Notifications {
            banners: vec![LiveBanner {
                note: held,
                life: BannerLife::Expires,
            }],
            ..Default::default()
        },
        ..Default::default()
    });

    peer.tell(&ToClient::Status(status(
        Mode::Paused,
        &[("k1", Grab::Active)],
    )));
    let replaced = next_notice(&mut rig.notices).await;
    let NotificationsCommand::Post(posting) = replaced else {
        panic!("{replaced:?}");
    };
    assert_eq!(posting.summary, notice_text(&Attention::Paused).summary);
    assert_eq!(posting.replaces, Some(NotificationId(7)));

    peer.tell(&ToClient::Status(running()));
    assert_eq!(
        next_notice(&mut rig.notices).await,
        NotificationsCommand::Dismiss(NotificationId(7))
    );
}

async fn next_notice(
    notices: &mut mpsc::UnboundedReceiver<NotificationsCommand>,
) -> NotificationsCommand {
    tokio::time::timeout(LONG, notices.recv())
        .await
        .unwrap_or_else(|_| panic!("no notice within {LONG:?}"))
        .unwrap_or_else(|| panic!("the notifications control closed"))
}

#[tokio::test(flavor = "multi_thread")]
async fn resume_reaches_the_daemon() {
    let mut rig = rig(None);
    let peer = connected(&mut rig).await;
    assert_eq!(peer.next(LONG), Some(focus("default")));
    rig.control
        .send(sill_model::keycap::KeycapCommand::Resume)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(peer.next(LONG), Some(ToDaemon::Resume));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_isolated_env_never_connects_and_stays_unknown() {
    let scratch = TempDir::new("keycap-isolated");
    let env = Env::isolated(Path::new(scratch.path()));
    assert_eq!(env.keycap_socket, None);
    let (_apps, apps_rx) = watch::channel(AppIndex::default());
    let (_wl, wl_rx) = watch::channel(CosmicWl::default());
    let (notifications, mut notices) = Control::channel();
    let (_notes, notes_rx) = watch::channel(NotesState::default());
    let handle = spawn::<KeycapService>(
        &tokio::runtime::Handle::current(),
        &env,
        KeycapLinks {
            cosmic_wl: wl_rx,
            apps: apps_rx,
            notifications,
            notes: notes_rx,
        },
    );
    for command in [
        sill_model::keycap::KeycapCommand::Retry,
        sill_model::keycap::KeycapCommand::Resume,
    ] {
        handle
            .control
            .send(command)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    tokio::time::sleep(SHORT).await;
    assert_eq!(*handle.state.borrow(), KeycapState::Unknown);
    assert!(notices.try_recv().is_err());
}
