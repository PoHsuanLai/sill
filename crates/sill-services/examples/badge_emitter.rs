//! A `LauncherEntry` emitter for exercising badges without a real app:
//!
//! ```sh
//! cargo run -p sill-services --example badge_emitter -- <desktop-id> \
//!     [--count N] [--progress 0..1] [--urgent]
//! ```
//!
//! Emits `Update` on the session bus every 200 ms until the process is killed, so the dock's
//! acceptance script and `tests/badges_live.rs` can drive a real badge and a real clear (the
//! bus notices the connection close and `badges::live` follows with `NameOwnerChanged`) without
//! a real app on the other end. Interface `com.canonical.Unity.LauncherEntry`, signal
//! `Update(s app_uri, a{sv})`.

use std::collections::HashMap;
use std::process::ExitCode;
use std::time::Duration;

use zbus::Connection;
use zbus::zvariant::Value;

const INTERFACE: &str = "com.canonical.Unity.LauncherEntry";
const MEMBER: &str = "Update";
const PATH: &str = "/org/sill/BadgeEmitter";
const REPEAT: Duration = Duration::from_millis(200);

/// Whether this run's badge should be bouncing the dock.
#[derive(Clone, Copy)]
enum Attention {
    Calm,
    Urgent,
}

/// What one run publishes, parsed once from argv. A key is present in the wire dictionary, and
/// marked visible, only when its flag was given — matching how a real app would only ever send
/// the properties it has something to say about.
struct Emitted {
    app_uri: String,
    count: Option<u64>,
    progress: Option<f64>,
    urgent: Attention,
}

#[tokio::main]
async fn main() -> ExitCode {
    let Some(emitted) = Emitted::from_args(std::env::args().skip(1)) else {
        eprintln!("usage: badge_emitter <desktop-id> [--count N] [--progress 0..1] [--urgent]");
        return ExitCode::FAILURE;
    };

    let Ok(connection) = Connection::session().await else {
        eprintln!("badge_emitter: could not connect to the session bus");
        return ExitCode::FAILURE;
    };

    let body = emitted.body();
    eprintln!(
        "badge_emitter: publishing {} every {}ms until killed",
        emitted.app_uri,
        REPEAT.as_millis()
    );
    loop {
        if let Err(error) = connection
            .emit_signal(None::<&str>, PATH, INTERFACE, MEMBER, &body)
            .await
        {
            eprintln!("badge_emitter: {error}");
            return ExitCode::FAILURE;
        }
        tokio::time::sleep(REPEAT).await;
    }
}

impl Emitted {
    /// Parses `<desktop-id> [--count N] [--progress F] [--urgent]`; `None` on a missing desktop
    /// id, an unrecognised flag or a flag missing its value.
    fn from_args(mut args: impl Iterator<Item = String>) -> Option<Emitted> {
        let desktop_id = args.next()?;
        let mut count = None;
        let mut progress = None;
        let mut urgent = Attention::Calm;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--count" => count = Some(args.next()?.parse().ok()?),
                "--progress" => progress = Some(args.next()?.parse().ok()?),
                "--urgent" => urgent = Attention::Urgent,
                _ => return None,
            }
        }
        Some(Emitted {
            app_uri: format!("application://{desktop_id}.desktop"),
            count,
            progress,
            urgent,
        })
    }

    /// The `Update` signal's own `(s app_uri, a{sv})` body.
    fn body(&self) -> (String, HashMap<String, Value<'static>>) {
        let mut props = HashMap::new();
        if let Some(count) = self.count {
            props.insert("count".to_owned(), Value::from(count as i64));
            props.insert("count-visible".to_owned(), Value::from(true));
        }
        if let Some(progress) = self.progress {
            props.insert("progress".to_owned(), Value::from(progress));
            props.insert("progress-visible".to_owned(), Value::from(true));
        }
        if let Attention::Urgent = self.urgent {
            props.insert("urgent".to_owned(), Value::from(true));
        }
        (self.app_uri.clone(), props)
    }
}
