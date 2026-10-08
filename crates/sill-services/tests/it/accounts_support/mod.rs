//! A stand-in accountd on a private bus: the `Manager` with its `NeedsReauth` signal and the
//! shell's list, and one `Account` whose `State` the test moves and whose `Reauthenticate` is
//! recorded and puts the account right, as a finished sign-in does.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sill_testkit::PrivateBus;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue};

pub const ACCOUNT: &str = "/org/quire/Accounts1/account/work";
const MANAGER: &str = "/org/quire/Accounts1";

/// What the stand-in was asked.
#[derive(Debug, Default)]
pub struct Record {
    /// Each `Reauthenticate`'s parent window.
    pub reauthenticated: Vec<String>,
}

pub type Shared = Arc<Mutex<Record>>;

/// The accounts the list names (object path and label).
struct Manager {
    listed: Vec<(OwnedObjectPath, String)>,
    /// How many more calls are refused with `AccessDenied`, as before accountd knows the caller.
    refusals: AtomicUsize,
}

#[zbus::interface(name = "org.quire.Accounts1.Manager")]
impl Manager {
    fn needing_reauth(&self) -> zbus::fdo::Result<Vec<(OwnedObjectPath, String)>> {
        let refused = self
            .refusals
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok();
        if refused {
            return Err(zbus::fdo::Error::AccessDenied(
                "caller not known yet".into(),
            ));
        }
        Ok(self.listed.clone())
    }

    #[zbus(signal)]
    async fn needs_reauth(emitter: &SignalEmitter<'_>, account: ObjectPath<'_>)
    -> zbus::Result<()>;
}

/// The Manager of an accountd that has no list for the shell: only the signal.
struct ManagerWithoutList;

#[zbus::interface(name = "org.quire.Accounts1.Manager")]
impl ManagerWithoutList {
    #[zbus(signal)]
    async fn needs_reauth(emitter: &SignalEmitter<'_>, account: ObjectPath<'_>)
    -> zbus::Result<()>;
}

struct Account {
    state: String,
    /// `ReauthReason`: empty, or `testing_app_expired`.
    reason: String,
    record: Shared,
}

#[zbus::interface(name = "org.quire.Accounts1.Account")]
impl Account {
    #[zbus(property)]
    fn label(&self) -> String {
        "Work mail".to_owned()
    }

    #[zbus(property)]
    fn state(&self) -> String {
        self.state.clone()
    }

    #[zbus(property)]
    fn reauth_reason(&self) -> String {
        self.reason.clone()
    }

    async fn reauthenticate(
        &mut self,
        parent_window: String,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> OwnedObjectPath {
        self.record
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .reauthenticated
            .push(parent_window);
        self.state = "ok".to_owned();
        let _ = self.state_changed(&emitter).await;
        OwnedObjectPath::try_from("/org/quire/Accounts1/request/1")
            .unwrap_or_else(|e| panic!("{e}"))
    }
}

fn manager(refusals: usize, listed: Vec<(&str, &str)>) -> Manager {
    Manager {
        listed: listed
            .into_iter()
            .map(|(path, label)| {
                (
                    OwnedObjectPath::try_from(path).unwrap_or_else(|e| panic!("{e}")),
                    label.to_owned(),
                )
            })
            .collect(),
        refusals: AtomicUsize::new(refusals),
    }
}

/// The stand-in on `bus`, holding `org.quire.Accounts1`.
pub struct FakeAccountd {
    pub connection: zbus::Connection,
    pub record: Shared,
}

/// Which `Manager` the stand-in serves.
pub enum Listing {
    /// Names `listed` as needing sign-in.
    Of(Vec<(&'static str, &'static str)>),
    /// Refuses the first `usize` calls with `AccessDenied`, then lists the `Of` way.
    Refusing(usize, Vec<(&'static str, &'static str)>),
    /// An accountd with no such call.
    None,
}

impl FakeAccountd {
    /// Start it with the account at `state`.
    pub async fn start(bus: &PrivateBus, state: &str, listing: Listing) -> FakeAccountd {
        let record = Shared::default();
        let builder = zbus::connection::Builder::address(bus.address.as_str())
            .unwrap_or_else(|e| panic!("{e}"))
            .serve_at(
                ACCOUNT,
                Account {
                    state: state.to_owned(),
                    reason: String::new(),
                    record: Arc::clone(&record),
                },
            )
            .unwrap_or_else(|e| panic!("{e}"));
        let builder = match listing {
            Listing::Of(listed) => builder.serve_at(MANAGER, manager(0, listed)),
            Listing::Refusing(refusals, listed) => {
                builder.serve_at(MANAGER, manager(refusals, listed))
            }
            Listing::None => builder.serve_at(MANAGER, ManagerWithoutList),
        }
        .unwrap_or_else(|e| panic!("{e}"));
        let connection = builder
            .name("org.quire.Accounts1")
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("the stand-in accountd: {e}"));
        FakeAccountd { connection, record }
    }

    /// The account stops working: its `State` changes and `NeedsReauth` is signalled.
    pub async fn refuse(&self) {
        self.set_state("needs_reauth").await;
        let emitter =
            SignalEmitter::new(&self.connection, MANAGER).unwrap_or_else(|e| panic!("{e}"));
        let path = ObjectPath::try_from(ACCOUNT).unwrap_or_else(|e| panic!("{e}"));
        // Both stand-in managers name the same interface, so the signal is the same.
        Manager::needs_reauth(&emitter, path)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The account stops working because Google signs an unverified app out every seven days:
    /// `ReauthReason` says `testing_app_expired` before the state changes.
    pub async fn refuse_testing_app_expired(&self) {
        let object = self
            .connection
            .object_server()
            .interface::<_, Account>(ACCOUNT)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        object.get_mut().await.reason = "testing_app_expired".to_owned();
        self.refuse().await;
    }

    /// The account is well again, without anyone pressing a button.
    pub async fn recover(&self) {
        self.set_state("ok").await;
    }

    async fn set_state(&self, state: &str) {
        let object = self
            .connection
            .object_server()
            .interface::<_, Account>(ACCOUNT)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        object.get_mut().await.state = state.to_owned();
        object
            .get()
            .await
            .state_changed(object.signal_emitter())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// How many times `Reauthenticate` was called.
    pub fn reauthentications(&self) -> usize {
        self.record
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .reauthenticated
            .len()
    }
}
