//! A stand-in for `sill auth-check`: it speaks the helper protocol
//! (`sill_services::auth::protocol`) without PAM, so `tests/auth_live.rs` drives the real
//! `PamHelper` against a child process and nobody's password is ever checked.
//!
//! `auth_helper accept SECRET` accepts exactly `SECRET` (with one `info` note first) and refuses
//! anything else (with one `error` note); `auth_helper hang` reads the request and never
//! answers; `auth_helper garbage` answers with a line that is not the protocol. It never prints
//! what it was sent.

use std::io::{Read, Write};
use std::process::ExitCode;
use std::time::Duration;

use sill_services::auth::protocol::{self, HelperLine};
use sill_services::auth::{Outcome, PamNote};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut request = Vec::with_capacity(4096);
    let read = std::io::stdin().lock().read_to_end(&mut request);
    let decoded = read
        .ok()
        .and_then(|_| protocol::decode_request(&request).ok());
    request.iter_mut().for_each(|byte| *byte = 0);
    let Some(request) = decoded else {
        return ExitCode::from(2);
    };
    let lines = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["accept", secret] if request.attempt.secret.expose() == *secret => vec![
            HelperLine::Note(PamNote::Info("stand-in helper".to_owned())),
            HelperLine::Done(Outcome::Accepted),
        ],
        ["accept", _] => vec![
            HelperLine::Note(PamNote::Error("wrong secret".to_owned())),
            HelperLine::Done(Outcome::Refused),
        ],
        ["hang"] => loop {
            std::thread::sleep(Duration::from_secs(60));
        },
        _ => {
            let _ = writeln!(std::io::stdout(), "maybe");
            return ExitCode::SUCCESS;
        }
    };
    let mut out = std::io::stdout().lock();
    for line in lines {
        let _ = writeln!(out, "{}", protocol::render_line(&line));
    }
    ExitCode::SUCCESS
}
