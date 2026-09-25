//! What a client hears from the notifications server: every signal of the interface on a
//! private bus, decoded into [`Heard`] in the order they arrive.

use std::pin::Pin;
use std::time::Duration;

use zbus::export::futures_core::Stream;
use zbus::message::Type as MessageType;
use zbus::{MatchRule, MessageStream};

use sill_services::notifications::BUS_NAME;

/// One signal heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    Closed { id: u32, reason: u32 },
    Invoked { id: u32, action: String },
    Token { id: u32, token: String },
}

/// A subscription to the interface's signals.
pub struct Signals {
    stream: MessageStream,
}

impl Signals {
    /// Listen on `connection` from now on.
    pub async fn listen(connection: &zbus::Connection) -> Signals {
        let rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface(BUS_NAME)
            .unwrap_or_else(|e| panic!("{e}"))
            .build();
        let stream = MessageStream::for_match_rule(rule, connection, None)
            .await
            .unwrap_or_else(|e| panic!("subscribing: {e}"));
        Signals { stream }
    }

    /// The next signal within `limit`.
    pub async fn next(&mut self, limit: Duration) -> Option<Heard> {
        let stream = &mut self.stream;
        let next = std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx));
        let message = tokio::time::timeout(limit, next).await.ok()??.ok()?;
        let header = message.header();
        let member = header.member()?.as_str().to_owned();
        let body = message.body();
        match member.as_str() {
            "NotificationClosed" => {
                let (id, reason): (u32, u32) = body.deserialize().ok()?;
                Some(Heard::Closed { id, reason })
            }
            "ActionInvoked" => {
                let (id, action): (u32, String) = body.deserialize().ok()?;
                Some(Heard::Invoked { id, action })
            }
            "ActivationToken" => {
                let (id, token): (u32, String) = body.deserialize().ok()?;
                Some(Heard::Token { id, token })
            }
            _ => None,
        }
    }
}
