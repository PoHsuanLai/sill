//! The session-bus half of badges: `LauncherEntry.Update` signals from any sender, decoded
//! into [`BadgeEvent`]s, and a sender's `NameOwnerChanged` departure clearing what it published
//! (plan LauncherEntry paragraph; interface `com.canonical.Unity.LauncherEntry`, signal
//! `Update(s app_uri, a{sv})`).

use std::collections::HashMap;
use std::pin::Pin;

use sill_launcher::AppId;
use tokio::sync::{mpsc, watch};
use zbus::export::futures_core::Stream;
use zbus::fdo::{DBusProxy, NameOwnerChanged};
use zbus::message::{Message, Type as MessageType};
use zbus::names::OwnedUniqueName;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageStream};

use super::BadgesCommand;
use super::model::{BadgeEvent, BadgeProp, Badges, parse_app_uri, reduce_badges};
use crate::cosmic_wl::Urgency;
use crate::units::{Permille, Visibility};

const INTERFACE: &str = "com.canonical.Unity.LauncherEntry";
const MEMBER: &str = "Update";

/// Connects to the session bus and runs until `commands` closes (the `Control` was dropped).
/// If the bus is unreachable, `tx` is left at `Badges::default()` — starting a service never
/// fails the shell (`error.rs`).
pub(super) async fn run(
    tx: watch::Sender<Badges>,
    mut commands: mpsc::UnboundedReceiver<BadgesCommand>,
) {
    let Ok(connection) = Connection::session().await else {
        return;
    };
    let Ok(rule) = update_rule() else {
        return;
    };
    let Ok(mut updates) = MessageStream::for_match_rule(rule, &connection, None).await else {
        return;
    };
    let Ok(dbus) = DBusProxy::new(&connection).await else {
        return;
    };
    let Ok(mut departures) = dbus.receive_name_owner_changed().await else {
        return;
    };

    // The sender that last published each app's badge, so its departure can clear it. Badges
    // itself only remembers app -> properties (plan's persisted shape); which connection is
    // currently speaking for an app is this task's own bookkeeping.
    let mut senders: HashMap<AppId, OwnedUniqueName> = HashMap::new();

    loop {
        tokio::select! {
            Some(message) = next(&mut updates) => {
                if let Ok(message) = message {
                    handle_update(&message, &tx, &mut senders);
                }
            }
            Some(signal) = next(&mut departures) => {
                handle_departure(&signal, &tx, &mut senders);
            }
            command = commands.recv() => match command {
                Some(BadgesCommand::Clear) => {
                    senders.clear();
                    apply(&tx, BadgeEvent::ClearedAll);
                }
                None => break,
            }
        }
    }
}

/// A bus-wide match on every `Update` signal of the interface, whoever sends it (plan: "zbus 5
/// match rule on the session bus").
fn update_rule() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(INTERFACE)?
        .member(MEMBER)?
        .build())
}

/// Decodes one `Update` message and folds it into `tx`, recording its sender so a later
/// departure can clear it. A message this crate cannot make sense of (no sender, no decodable
/// `app_uri`) is dropped rather than treated as a badge for nobody.
fn handle_update(
    message: &Message,
    tx: &watch::Sender<Badges>,
    senders: &mut HashMap<AppId, OwnedUniqueName>,
) {
    let Some(sender) = message.header().sender().map(|s| s.to_owned().into()) else {
        return;
    };
    let Some(event) = decode_update(message) else {
        return;
    };
    if let BadgeEvent::Update { app, .. } = &event {
        senders.insert(app.clone(), sender);
    }
    apply(tx, event);
}

/// The `BadgeEvent` an `Update(s app_uri, a{sv})` message carries, or `None` if the body is not
/// that shape at all or `app_uri` names no app (`parse_app_uri`).
fn decode_update(message: &Message) -> Option<BadgeEvent> {
    let (app_uri, dict) = message
        .body()
        .deserialize::<(String, HashMap<String, OwnedValue>)>()
        .ok()?;
    let app = parse_app_uri(&app_uri)?;
    Some(BadgeEvent::Update {
        app,
        props: decode_props(dict),
    })
}

/// Every property whose value matched the type the interface promises for its key. A key that
/// is unknown, or holds the wrong type, is dropped rather than failing the whole message —
/// `reduce_badges` already treats an absent key as "unchanged", which is what a dropped one
/// becomes here.
fn decode_props(dict: HashMap<String, OwnedValue>) -> Vec<BadgeProp> {
    dict.into_iter()
        .filter_map(|(key, value)| decode_prop(&key, value))
        .collect()
}

fn decode_prop(key: &str, value: OwnedValue) -> Option<BadgeProp> {
    match key {
        "count" => i64::try_from(value).ok().map(BadgeProp::Count),
        "count-visible" => bool::try_from(value)
            .ok()
            .map(|shown| BadgeProp::CountVisibility(visibility_of(shown))),
        "progress" => f64::try_from(value)
            .ok()
            .map(|progress| BadgeProp::Progress(permille_of(progress))),
        "progress-visible" => bool::try_from(value)
            .ok()
            .map(|shown| BadgeProp::ProgressVisibility(visibility_of(shown))),
        "urgent" => bool::try_from(value)
            .ok()
            .map(|urgent| BadgeProp::Urgency(urgency_of(urgent))),
        _ => None,
    }
}

fn visibility_of(shown: bool) -> Visibility {
    if shown {
        Visibility::Shown
    } else {
        Visibility::Hidden
    }
}

fn urgency_of(urgent: bool) -> Urgency {
    if urgent {
        Urgency::Urgent
    } else {
        Urgency::Calm
    }
}

/// `progress` 0..1 as thousandths; a negative or NaN value floors to zero the same way the
/// round-then-cast already does (`reduce_badges`'s `apply` still clamps the top end).
fn permille_of(progress: f64) -> Permille {
    Permille((progress * 1000.0).round() as u32)
}

/// Clears every app the departing connection last published, when its unique name is truly
/// gone (`new_owner` empty). A name *transfer* between two still-live connections leaves
/// badges alone: only a unique name losing its owner means the connection itself hung up.
fn handle_departure(
    signal: &NameOwnerChanged,
    tx: &watch::Sender<Badges>,
    senders: &mut HashMap<AppId, OwnedUniqueName>,
) {
    let Ok(args) = signal.args() else {
        return;
    };
    if args.new_owner().is_some() {
        return;
    }
    let left = args.name().as_str();
    let gone: Vec<AppId> = senders
        .iter()
        .filter(|(_, sender)| sender.as_str() == left)
        .map(|(app, _)| app.clone())
        .collect();
    for app in gone {
        senders.remove(&app);
        apply(tx, BadgeEvent::Cleared(app));
    }
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<Badges>, event: BadgeEvent) {
    let next = reduce_badges(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

/// Polls one `Unpin` stream for its next item, for use as a `tokio::select!` branch.
async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use zbus::zvariant::{OwnedValue, Value};

    use super::{decode_props, permille_of};
    use crate::badges::model::BadgeProp;
    use crate::units::{Permille, Visibility};

    fn dict(pairs: Vec<(&str, Value<'static>)>) -> HashMap<String, OwnedValue> {
        pairs
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_owned(),
                    OwnedValue::try_from(v).expect("ownable value"),
                )
            })
            .collect()
    }

    /// A decode table name, the raw `a{sv}` it stands for, and the props it must yield —
    /// unordered, since a `HashMap` iterates in no particular order.
    struct Case {
        name: &'static str,
        dict: HashMap<String, OwnedValue>,
        want: Vec<BadgeProp>,
    }

    #[test]
    fn well_typed_and_unknown_keys() {
        let cases = vec![
            Case {
                name: "every recognised key, well typed",
                dict: dict(vec![
                    ("count", Value::from(7_i64)),
                    ("count-visible", Value::from(true)),
                    ("progress", Value::from(0.5_f64)),
                    ("progress-visible", Value::from(true)),
                    ("urgent", Value::from(true)),
                ]),
                want: vec![
                    BadgeProp::Count(7),
                    BadgeProp::CountVisibility(Visibility::Shown),
                    BadgeProp::Progress(Permille(500)),
                    BadgeProp::ProgressVisibility(Visibility::Shown),
                    BadgeProp::Urgency(crate::cosmic_wl::Urgency::Urgent),
                ],
            },
            Case {
                name: "a partial dictionary decodes the keys it has",
                dict: dict(vec![("count", Value::from(3_i64))]),
                want: vec![BadgeProp::Count(3)],
            },
            Case {
                name: "an unknown key is dropped, not an error",
                dict: dict(vec![
                    ("count", Value::from(1_i64)),
                    ("icon-name", Value::from("mail-symbolic")),
                ]),
                want: vec![BadgeProp::Count(1)],
            },
            Case {
                name: "a mistyped key is dropped, its neighbours survive",
                dict: dict(vec![
                    ("count", Value::from("not a number")),
                    ("urgent", Value::from(true)),
                ]),
                want: vec![BadgeProp::Urgency(crate::cosmic_wl::Urgency::Urgent)],
            },
            Case {
                name: "a bool where a double is expected is dropped",
                dict: dict(vec![("progress", Value::from(true))]),
                want: vec![],
            },
            Case {
                name: "an int where a bool is expected is dropped",
                dict: dict(vec![("count-visible", Value::from(1_i64))]),
                want: vec![],
            },
            Case {
                name: "an empty dictionary decodes to nothing",
                dict: dict(vec![]),
                want: vec![],
            },
        ];
        for case in cases {
            let mut got = decode_props(case.dict);
            let mut want = case.want;
            got.sort_by_key(prop_rank);
            want.sort_by_key(prop_rank);
            assert_eq!(got, want, "{}", case.name);
        }
    }

    fn prop_rank(prop: &BadgeProp) -> u8 {
        match prop {
            BadgeProp::Count(_) => 0,
            BadgeProp::CountVisibility(_) => 1,
            BadgeProp::Progress(_) => 2,
            BadgeProp::ProgressVisibility(_) => 3,
            BadgeProp::Urgency(_) => 4,
        }
    }

    #[test]
    fn progress_floors_negative_and_nan_to_zero() {
        const CASES: &[(f64, Permille)] = &[
            (0.0, Permille(0)),
            (0.5, Permille(500)),
            (1.0, Permille(1000)),
            (-0.3, Permille(0)),
            (f64::NAN, Permille(0)),
        ];
        for &(progress, want) in CASES {
            assert_eq!(permille_of(progress), want, "{progress}");
        }
    }
}
