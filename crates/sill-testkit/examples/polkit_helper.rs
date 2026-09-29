//! A stand-in for polkit's `polkit-agent-helper-1`: speaks the setuid helper's line protocol and
//! accepts `sill-fake` (`FAKE_SECRET`) and nothing else. It runs no PAM, reads no password database
//! and tells no polkitd anything, so the live test and `dev/accept-polkit.sh` can drive sill's
//! agent through a real child process.
//!
//! Protocol: the user name is the argument; the cookie is the first stdin line; it writes
//! `PAM_TEXT_INFO`, then `PAM_PROMPT_ECHO_OFF Password: `, reads one line, and ends with
//! `SUCCESS` or with `PAM_ERROR_MSG` and `FAILURE`. What was typed is never printed.
//!
//! Usage: `polkit_helper <user>` (the agent runs it; `sill run --polkit-helper` names it).

use std::io::{BufRead, Write};

use sill_model::polkit_agent::FAKE_SECRET;

fn main() {
    let user = std::env::args().nth(1).unwrap_or_default();
    let stdin = std::io::stdin();
    let mut lines = stdin.lock();
    let mut cookie = String::new();
    if lines.read_line(&mut cookie).unwrap_or(0) == 0 || user.is_empty() {
        say("FAILURE");
        std::process::exit(1);
    }
    say(&format!("PAM_TEXT_INFO stand-in helper for {user}"));
    say("PAM_PROMPT_ECHO_OFF Password: ");
    let mut answer = String::new();
    let _ = lines.read_line(&mut answer);
    let right = answer.trim_end_matches(['\n', '\r']) == FAKE_SECRET;
    wipe(answer);
    eprintln!(
        "polkit_helper: cookie {} {}",
        cookie.trim_end(),
        if right { "accepted" } else { "refused" }
    );
    if right {
        say("SUCCESS");
    } else {
        say("PAM_ERROR_MSG Sorry, that didn't work.");
        say("FAILURE");
    }
}

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn wipe(text: String) {
    let mut bytes = text.into_bytes();
    bytes.iter_mut().for_each(|b| *b = 0);
    std::hint::black_box(&bytes);
}
