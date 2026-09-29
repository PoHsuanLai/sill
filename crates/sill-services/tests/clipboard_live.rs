//! The clipboard history against a compositor: the live test drives the nested cosmic-comp by
//! socket path with made-up text and a made-up image; the other test needs no compositor. Nothing
//! read back is printed: a mismatch says only that it differs.

mod clipboard_support;

use std::time::Duration;

use sill_launcher::ClipKey;
use sill_services::clipboard::ClipboardService;
use sill_services::clipboard::{
    ClipContent, ClipPolicy, ClipProtocol, Clipboard, ClipboardCommand, EntryCap, HistoryMode,
    ImageCap,
};
use sill_services::control::Control;
use sill_services::env::{Env, WaylandRoute};
use sill_services::service::spawn;
use tokio::runtime::Runtime;
use tokio::sync::watch;

use clipboard_support::{
    clear, copy_png, copy_text, nested_socket, paste_text, png, secret_source,
};

const WITHIN: Duration = Duration::from_secs(5);

/// A made-up string no one has in their clipboard.
const MADE_UP: &str = "m9-live-lantern-4417 made-up words";

fn memory() -> ClipboardCommand {
    ClipboardCommand::Policy(ClipPolicy {
        mode: HistoryMode::Memory,
        cap: EntryCap(10),
        images: ImageCap(8 << 20),
    })
}

/// The service on the compositor `display` names; nothing else reached.
fn start(
    rt: &Runtime,
    display: WaylandRoute,
) -> (watch::Receiver<Clipboard>, Control<ClipboardCommand>) {
    let env = Env {
        wayland: display,
        ..Env::isolated(std::path::Path::new("/nonexistent/sill-clipboard-live"))
    };
    let service = spawn::<ClipboardService>(rt.handle(), &env, ());
    (service.state, service.control)
}

fn runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a runtime")
}

/// Wait until `ready` holds for the published history; the panic message is `what` alone.
fn wait(
    rt: &Runtime,
    history: &mut watch::Receiver<Clipboard>,
    what: &str,
    ready: impl Fn(&Clipboard) -> bool,
) {
    let held = rt.block_on(async {
        tokio::time::timeout(WITHIN, history.wait_for(|h| ready(h)))
            .await
            .is_ok_and(|seen| seen.is_ok())
    });
    let shown = history.borrow();
    assert!(
        held,
        "{what}: not within {WITHIN:?} ({:?}, {} entries)",
        shown.status,
        shown.entries.len()
    );
}

fn is_text(content: &ClipContent, want: &str) -> bool {
    matches!(content, ClipContent::Text(text) if *text.0 == *want)
}

#[test]
fn without_a_compositor_history_is_on_but_not_connected() {
    let rt = runtime();
    let missing = std::env::temp_dir().join("sill-clipboard-no-such-socket");
    let (mut history, control) = start(&rt, WaylandRoute::Socket(missing));
    control.send(memory()).expect("the service listens");
    wait(&rt, &mut history, "history on", |h| {
        h.status.mode == HistoryMode::Memory
    });
    std::thread::sleep(Duration::from_millis(300));
    let shown = history.borrow().clone();
    assert_eq!(shown.status.protocol, ClipProtocol::NotConnected);
    assert!(shown.entries.is_empty());
}

#[test]
#[ignore = "needs the nested compositor (dev/nested.sh) and wl-clipboard"]
fn the_history_follows_the_nested_clipboard() {
    let socket = nested_socket();
    let rt = runtime();
    let (mut history, control) = start(&rt, WaylandRoute::Socket(socket.clone()));
    control.send(memory()).expect("the service listens");
    wait(&rt, &mut history, "connected", |h| {
        h.status.protocol != ClipProtocol::NotConnected
    });
    let protocol = history.borrow().status.protocol;
    eprintln!("clipboard_live: the nested compositor gave {protocol:?}");
    assert!(matches!(protocol, ClipProtocol::Ext | ClipProtocol::Wlr));

    // A copy is kept, whole.
    copy_text(&socket, MADE_UP);
    wait(&rt, &mut history, "one text entry", |h| {
        h.entries.len() == 1 && is_text(&h.entries[0].content, MADE_UP)
    });
    let key = history.borrow().entries[0].key;

    // A password manager's copy is never kept.
    let mut secret = secret_source(&socket);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        history.borrow().entries.len(),
        1,
        "the marked copy was skipped"
    );

    // Offering the entry back makes it the selection again, without a second entry.
    control
        .send(ClipboardCommand::Offer(key))
        .expect("the service listens");
    let pasted = (0..50).find_map(|_| {
        std::thread::sleep(Duration::from_millis(100));
        (paste_text(&socket) == MADE_UP.as_bytes()).then_some(())
    });
    assert!(pasted.is_some(), "wl-paste did not read the offered entry");
    let _ = secret.wait();
    std::thread::sleep(Duration::from_millis(300));
    let shown = history.borrow().clone();
    assert_eq!(shown.entries.len(), 1, "offering back does not duplicate");
    assert_eq!(shown.entries[0].key, key);

    // An image is kept with its size.
    copy_png(&socket, &png(3, 2));
    wait(&rt, &mut history, "an image entry", |h| {
        matches!(h.entries.first().map(|e| &e.content),
            Some(ClipContent::Image(i)) if (i.width, i.height) == (3, 2) && i.mime.0 == "image/png")
    });
    let image: ClipKey = history.borrow().entries[0].key;

    // Forget, clear, and off.
    control
        .send(ClipboardCommand::Forget(image))
        .expect("listens");
    wait(&rt, &mut history, "forgotten", |h| h.entries.len() == 1);
    control.send(ClipboardCommand::Clear).expect("listens");
    wait(&rt, &mut history, "cleared", |h| h.entries.is_empty());
    copy_text(&socket, "m9-live-second made-up copy");
    wait(&rt, &mut history, "kept again", |h| h.entries.len() == 1);
    control
        .send(ClipboardCommand::Policy(ClipPolicy::default()))
        .expect("listens");
    wait(&rt, &mut history, "off", |h| {
        h.status.mode == HistoryMode::Off
            && h.status.protocol == ClipProtocol::NotConnected
            && h.entries.is_empty()
    });
    clear(&socket);
}
