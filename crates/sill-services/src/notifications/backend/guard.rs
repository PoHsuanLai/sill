//! The gate in front of the served object: a `Notify` whose body is longer than any notification
//! could be is answered `LimitsExceeded` before one byte of it is read into strings, hints or
//! pixels. The object's own methods (`dbus.rs`) read their arguments as the bus library hands
//! them over, and by then the whole body is already parsed; so the served object is this wrapper,
//! which looks at the call's length first and hands every call that passes, and every other
//! method, property and introspection request, to [`Server`] untouched.
//!
//! The bus library's `Interface` trait is the only place a call is seen before its arguments are
//! read, so it is implemented here by hand, once, for this wrapper alone.

use std::collections::HashMap;
use std::fmt::Write;

use sill_model::notifications::NOTIFY_CALL_MAX_BYTES;
use zbus::export::async_trait::async_trait;
use zbus::message::{Header, Message};
use zbus::names::{InterfaceName, MemberName};
use zbus::object_server::{DispatchResult2, Interface, SignalEmitter};
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, ObjectServer, fdo};

use super::dbus::Server;

/// The method the gate looks at.
const NOTIFY: &str = "Notify";

/// The served object: [`Server`] behind the gate.
#[derive(Debug)]
pub struct Guarded {
    inner: Server,
}

impl Guarded {
    /// `inner`, served behind the gate.
    pub fn new(inner: Server) -> Self {
        Self { inner }
    }
}

/// Whether a body of `bytes` is longer than a `Notify` may be.
fn too_long(bytes: u32) -> bool {
    !usize::try_from(bytes).is_ok_and(|bytes| bytes <= NOTIFY_CALL_MAX_BYTES)
}

/// Whether `msg` is a `Notify` too long to read.
fn refused(msg: &Message, name: &MemberName<'_>) -> bool {
    name.as_str() == NOTIFY && too_long(msg.header().primary().body_len())
}

#[async_trait]
impl Interface for Guarded {
    fn name() -> InterfaceName<'static> {
        Server::name()
    }

    fn spawn_tasks_for_methods(&self) -> bool {
        self.inner.spawn_tasks_for_methods()
    }

    async fn get(
        &self,
        property_name: &str,
        server: &ObjectServer,
        connection: &Connection,
        header: Option<&Header<'_>>,
        emitter: &SignalEmitter<'_>,
    ) -> Option<fdo::Result<OwnedValue>> {
        self.inner
            .get(property_name, server, connection, header, emitter)
            .await
    }

    async fn get_all(
        &self,
        object_server: &ObjectServer,
        connection: &Connection,
        header: Option<&Header<'_>>,
        emitter: &SignalEmitter<'_>,
    ) -> fdo::Result<HashMap<String, OwnedValue>> {
        self.inner
            .get_all(object_server, connection, header, emitter)
            .await
    }

    fn set<'call>(
        &'call self,
        property_name: &'call str,
        value: &'call Value<'_>,
        object_server: &'call ObjectServer,
        connection: &'call Connection,
        header: Option<&'call Header<'_>>,
        emitter: &'call SignalEmitter<'_>,
    ) -> DispatchResult2<'call> {
        self.inner.set(
            property_name,
            value,
            object_server,
            connection,
            header,
            emitter,
        )
    }

    async fn set_mut(
        &mut self,
        property_name: &str,
        value: &Value<'_>,
        object_server: &ObjectServer,
        connection: &Connection,
        header: Option<&Header<'_>>,
        emitter: &SignalEmitter<'_>,
    ) -> Option<fdo::Result<()>> {
        self.inner
            .set_mut(
                property_name,
                value,
                object_server,
                connection,
                header,
                emitter,
            )
            .await
    }

    fn call<'call>(
        &'call self,
        server: &'call ObjectServer,
        connection: &'call Connection,
        msg: &'call Message,
        name: MemberName<'call>,
    ) -> DispatchResult2<'call> {
        if refused(msg, &name) {
            return DispatchResult2::new_async(connection, msg, async {
                Err::<(), _>(fdo::Error::LimitsExceeded(
                    "that notification is too large".to_owned(),
                ))
            });
        }
        self.inner.call(server, connection, msg, name)
    }

    fn call_mut<'call>(
        &'call mut self,
        server: &'call ObjectServer,
        connection: &'call Connection,
        msg: &'call Message,
        name: MemberName<'call>,
    ) -> DispatchResult2<'call> {
        self.inner.call_mut(server, connection, msg, name)
    }

    fn introspect_to_writer(&self, writer: &mut dyn Write, level: usize) {
        self.inner.introspect_to_writer(writer, level);
    }
}

#[cfg(test)]
mod tests {
    use sill_model::notifications::NOTIFY_CALL_MAX_BYTES;

    use super::too_long;

    #[test]
    fn a_body_is_too_long_only_past_the_ceiling() {
        let ceiling = u32::try_from(NOTIFY_CALL_MAX_BYTES).unwrap_or(u32::MAX);
        let cases = [
            ("empty", 0, false),
            ("at the ceiling", ceiling, false),
            ("one over", ceiling + 1, true),
            ("two mebibytes", 2 << 20, true),
        ];
        for (case, bytes, want) in cases {
            assert_eq!(too_long(bytes), want, "{case}");
        }
    }
}
