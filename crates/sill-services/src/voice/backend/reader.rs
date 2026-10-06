//! The utterance's event stream: length-prefixed envelopes read off a socket and told to the loop
//! one by one, until it closes. A frame that does not decode is skipped (the next one is
//! whole); a frame over the cap cannot be skipped, so the stream is closed and the loop told.

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;
use voice_wire::{Unframed, VoiceEvent as Frame, unframe};

use super::run::{Inbound, Serial};

/// Read `stream` to its end, telling `inbound` each frame under `serial`.
pub(super) async fn read(
    mut stream: UnixStream,
    serial: Serial,
    inbound: UnboundedSender<Inbound>,
) {
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
        }
        loop {
            match unframe::<Frame>(&buffer) {
                Unframed::Partial => break,
                Unframed::Frame { body, used } => {
                    buffer.drain(..used);
                    if inbound.send(Inbound::Frame(serial, body)).is_err() {
                        return;
                    }
                }
                Unframed::Malformed { used } => {
                    buffer.drain(..used);
                }
                Unframed::TooLong { len } => {
                    eprintln!("sill: voice: a frame of {len} bytes is over the cap; closing");
                    let _ = inbound.send(Inbound::TooLong(serial));
                    return;
                }
            }
        }
    }
    let _ = inbound.send(Inbound::Closed(serial));
}
