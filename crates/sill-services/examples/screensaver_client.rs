//! A video player's half of `org.freedesktop.ScreenSaver` on a private bus (sill FINDINGS "sill
//! idle fill"): it calls `Inhibit`, holds the cookie while it runs, and on command calls
//! `UnInhibit`. Closing its stdin ends it (so its unique name leaves the bus, which also ends
//! the inhibit). `dev/accept-idle.sh` runs it to check that an inhibit holds sill's idle off.
//!
//! Commands, one per stdin line: `inhibit`, `uninhibit`, `active`. Output: `cookie <n>`,
//! `released <n>`, `active <b>`, or `failed <why>`.
//!
//! Usage: `cargo run -p sill-services --example screensaver_client -- <bus address>`

use std::io::Write;

use zbus::Connection;

const NAME: &str = "org.freedesktop.ScreenSaver";
const PATH: &str = "/org/freedesktop/ScreenSaver";

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage: screensaver_client <private bus address>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(run(address));
}

async fn call<B>(connection: &Connection, method: &str, body: &B) -> zbus::Result<zbus::Message>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    connection
        .call_method(Some(NAME), PATH, Some(NAME), method, body)
        .await
}

async fn run(address: String) {
    let connection = zbus::connection::Builder::address(address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
    let mut cookie: Option<u32> = None;
    for line in std::io::stdin().lines().map_while(Result::ok) {
        match line.trim() {
            "inhibit" => {
                let body = ("accept-idle", "playing a video");
                match call(&connection, "Inhibit", &body).await {
                    Ok(reply) => match reply.body().deserialize::<u32>() {
                        Ok(n) => {
                            cookie = Some(n);
                            say(&format!("cookie {n}"));
                        }
                        Err(error) => say(&format!("failed {error}")),
                    },
                    Err(error) => say(&format!("failed {error}")),
                }
            }
            "uninhibit" => {
                let n = cookie.take().unwrap_or(0);
                match call(&connection, "UnInhibit", &(n,)).await {
                    Ok(_) => say(&format!("released {n}")),
                    Err(error) => say(&format!("failed {error}")),
                }
            }
            "active" => match call(&connection, "GetActive", &()).await {
                Ok(reply) => match reply.body().deserialize::<bool>() {
                    Ok(active) => say(&format!("active {active}")),
                    Err(error) => say(&format!("failed {error}")),
                },
                Err(error) => say(&format!("failed {error}")),
            },
            other => eprintln!("screensaver_client: unknown command {other:?}"),
        }
    }
}
