//! A stand-in for accountd on a private bus: it owns `org.quire.Accounts1` there and drives sill's
//! accounts sheet (`org.quire.AccountsSheet1`) the way accountd does, so `dev/accept/accounts.sh`
//! can play the consent question and the add-account steps without accountd.
//!
//! The address is an argument, never the session bus, so this cannot reach the person's accounts
//! by accident. Commands, one per stdin line: `open <handle> <view.json>`, `update <handle>
//! <view.json>`, `close <handle>`, and `forge-open <handle> <view.json>`, which sends the same
//! `Open` from a second connection that does not own `org.quire.Accounts1` (it must be refused).
//! Output, one per stdout line: `ready`, `opened|updated|closed <handle>`, `refused <handle>
//! <why>`, `forged <handle> refused|ACCEPTED`, and `input <handle> <json>` for each answer the
//! sheet sends.
//!
//! Usage: `cargo run -p sill-testkit --example accountd_standin -- <bus address>`

use std::io::Write;
use std::pin::Pin;

use porter_dbus::accounts::AccountsSheetProxy;
use tokio::sync::mpsc;
use zbus::export::futures_core::Stream;
use zbus::{Connection, MatchRule, MessageStream};

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage: accountd_standin <private bus address>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(serve(address));
}

async fn connect(address: &str, name: Option<&str>) -> Connection {
    let builder = zbus::connection::Builder::address(address).unwrap_or_else(|e| panic!("{e}"));
    let builder = match name {
        Some(name) => builder
            .name(name.to_owned())
            .unwrap_or_else(|e| panic!("{e}")),
        None => builder,
    };
    builder
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to {address}: {e}"))
}

async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

async fn serve(address: String) {
    let accountd = connect(&address, Some("org.quire.Accounts1")).await;
    let forger = connect(&address, None).await;
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.quire.AccountsSheet1")
        .and_then(|b| b.member("Input"))
        .map(|b| b.build())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut inputs = MessageStream::for_match_rule(rule, &accountd, None)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let (proxy, forged) = (host(&accountd).await, host(&forger).await);
    say("ready");
    let mut lines = stdin_lines();
    loop {
        tokio::select! {
            Some(line) = lines.recv() => command(&proxy, &forged, &line).await,
            Some(Ok(message)) = next(&mut inputs) => {
                if let Ok((handle, input)) = message.body().deserialize::<(String, String)>() {
                    say(&format!("input {handle} {input}"));
                }
            }
            else => return,
        }
    }
}

async fn host(connection: &Connection) -> AccountsSheetProxy<'static> {
    AccountsSheetProxy::new(connection)
        .await
        .unwrap_or_else(|e| panic!("{e}"))
}

fn view_file(path: &str) -> String {
    std::fs::read_to_string(path)
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

async fn command(proxy: &AccountsSheetProxy<'_>, forged: &AccountsSheetProxy<'_>, line: &str) {
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        ["open", handle, file] => {
            report(
                "opened",
                handle,
                proxy.open(handle, "", &view_file(file)).await,
            );
        }
        ["update", handle, file] => {
            report(
                "updated",
                handle,
                proxy.update(handle, &view_file(file)).await,
            );
        }
        ["close", handle] => report("closed", handle, proxy.close(handle).await),
        ["forge-open", handle, file] => {
            let verdict = match forged.open(handle, "", &view_file(file)).await {
                Err(_) => "refused",
                Ok(()) => "ACCEPTED",
            };
            say(&format!("forged {handle} {verdict}"));
        }
        _ => eprintln!("accountd_standin: unknown command {line:?}"),
    }
}

fn report(word: &str, handle: &str, result: zbus::Result<()>) {
    match result {
        Ok(()) => say(&format!("{word} {handle}")),
        Err(error) => say(&format!("refused {handle} {error}")),
    }
}

fn stdin_lines() -> mpsc::UnboundedReceiver<String> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    rx
}
