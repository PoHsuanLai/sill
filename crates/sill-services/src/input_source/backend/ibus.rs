//! IBus over its own bus (no crate for three methods). The engines cycled are the active ones,
//! the list `ibus engine` and its hotkey go through.

use sill_model::input_source::{InputRefusal, address_in, next_of};
use zbus::proxy;
use zbus::zvariant::{OwnedValue, Value};

use crate::env::IbusRoute;

#[proxy(
    interface = "org.freedesktop.IBus",
    default_service = "org.freedesktop.IBus",
    default_path = "/org/freedesktop/IBus"
)]
trait Ibus {
    /// The engine descriptions the person turned on.
    fn list_active_engines(&self) -> zbus::Result<Vec<OwnedValue>>;

    /// The engine in use; an error when none is.
    fn get_global_engine(&self) -> zbus::Result<OwnedValue>;

    fn set_global_engine(&self, name: &str) -> zbus::Result<()>;
}

/// A connection to IBus's bus, when one answers. Address files of IBus sessions that ended are
/// tried and passed over.
pub(super) async fn connect(route: &IbusRoute) -> Option<zbus::Connection> {
    for address in addresses(route) {
        let Ok(builder) = zbus::connection::Builder::address(address.as_str()) else {
            continue;
        };
        if let Ok(connection) = builder.build().await {
            return Some(connection);
        }
    }
    None
}

/// The addresses to try, most likely first. Reads only.
fn addresses(route: &IbusRoute) -> Vec<String> {
    match route {
        IbusRoute::Absent => Vec::new(),
        IbusRoute::Address(address) => vec![address.clone()],
        IbusRoute::Files(dir) => {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return Vec::new();
            };
            let mut files: Vec<_> = entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let modified = entry.metadata().ok()?.modified().ok()?;
                    Some((modified, entry.path()))
                })
                .collect();
            files.sort_by_key(|a| std::cmp::Reverse(a.0));
            files
                .into_iter()
                .filter_map(|(_, path)| std::fs::read_to_string(path).ok())
                .filter_map(|text| address_in(&text).map(str::to_owned))
                .collect()
        }
    }
}

/// Switch IBus to the next of its active engines; the name it was told.
pub(super) async fn switch(connection: &zbus::Connection) -> Result<String, InputRefusal> {
    let failed = |error: zbus::Error| InputRefusal::Failed(error.to_string());
    let proxy = IbusProxy::new(connection).await.map_err(failed)?;
    let active = proxy.list_active_engines().await.map_err(failed)?;
    let names: Vec<String> = active
        .iter()
        .filter_map(|engine| engine_name(engine))
        .collect();
    let current = proxy
        .get_global_engine()
        .await
        .ok()
        .and_then(|engine| engine_name(&engine))
        .unwrap_or_default();
    let next = next_of(&current, &names)
        .ok_or(InputRefusal::NothingToSwitchTo)?
        .to_owned();
    proxy.set_global_engine(&next).await.map_err(failed)?;
    Ok(next)
}

/// The name in an engine description: a structure that begins with its type's name and its
/// attachments, then the engine's name.
fn engine_name(description: &Value<'_>) -> Option<String> {
    match description {
        Value::Value(inner) => engine_name(inner),
        Value::Structure(structure) => match structure.fields().get(2)? {
            Value::Str(name) => Some(name.to_string()),
            _ => None,
        },
        _ => None,
    }
}
