//! The utterance's event stream: length-prefixed envelopes read off a socket and told to the loop
//! one by one, until it closes.

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;
use voice_wire::{VoiceEvent as Frame, unframe};

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
        while let Some((frame, used)) = unframe::<Frame>(&buffer) {
            buffer.drain(..used);
            if inbound.send(Inbound::Frame(serial, frame)).is_err() {
                return;
            }
        }
    }
    let _ = inbound.send(Inbound::Closed(serial));
}
