//! Session commands: lock, log out, suspend, restart, shut down, reload.
//!
//! `query` lists every command whose name (or an alias) matches, except that Suspend, Restart
//! and Shut Down are left off when logind says it cannot do them right now (`Manager.Can*`,
//! read-only) or logind is unreachable at all — the "reports `Unavailable` cleanly" half of this
//! wave's brief; Lock and Reload need no such check (`loginctl lock-session` and the daemon's
//! own reload need no session permission that can vary at runtime). `activate` stays pure
//! (`sill_launcher::Provider`'s own contract): it never calls `loginctl` or logind itself, only
//! says which [`SystemCommand`] was chosen; the daemon carries it out.

use std::sync::OnceLock;

use logind_zbus::manager::{IsSupported, ManagerProxy};
use sill_launcher::{
    Action, ActionId, Activation, Batch, Choice, FrecencyScore, IconRef, Item, ItemId, Needle,
    Provider, ProviderKind, Query, RankWeights, ResultSink, SystemCommand, matching, rank_text,
};

/// One row's static shape: its command, title, search aliases, glyph (design/08-ICONS.md §1.6
/// has only `lock` and `power`; every power-menu row but Lock shares `power`, and Reload —
/// not a power-menu action — shows the provider's own glyph instead of a made-up one), and
/// whether choosing it outright is safe or needs [`Risk::Confirm`].
struct Row {
    command: SystemCommand,
    id: &'static str,
    title: &'static str,
    aliases: &'static [&'static str],
    glyph: Option<&'static str>,
    risk: Risk,
}

/// Whether a command may run straight from Enter, or must be confirmed first. Design/06 §20.2
/// gives the launcher exactly one lever for this in the frozen `Item`/`Choice` shape: the
/// actions menu (Ctrl+K / Alt+K). A [`Risk::Confirm`] row's `Choice::Primary` is
/// [`Activation::Nothing`]; its one action (id `"confirm"`) is the real command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Risk {
    Direct,
    Confirm,
}

/// Log Out, Restart and Shut Down close or interrupt every running app; Suspend and Reload do
/// not (a consequence of interrupting nothing, not of being uninteresting — Suspend is
/// reversible and instantaneous, matching how macOS and GNOME both treat it).
const ROWS: &[Row] = &[
    Row {
        command: SystemCommand::Lock,
        id: "lock",
        title: "Lock",
        aliases: &["Lock Screen"],
        glyph: Some("lock"),
        risk: Risk::Direct,
    },
    Row {
        command: SystemCommand::LogOut,
        id: "log_out",
        title: "Log Out",
        aliases: &["Sign Out"],
        glyph: Some("power"),
        risk: Risk::Confirm,
    },
    Row {
        command: SystemCommand::Suspend,
        id: "suspend",
        title: "Suspend",
        aliases: &["Sleep"],
        glyph: Some("power"),
        risk: Risk::Direct,
    },
    Row {
        command: SystemCommand::Restart,
        id: "restart",
        title: "Restart",
        aliases: &["Reboot"],
        glyph: Some("power"),
        risk: Risk::Confirm,
    },
    Row {
        command: SystemCommand::ShutDown,
        id: "shut_down",
        title: "Shut Down",
        aliases: &["Power Off", "Shutdown"],
        glyph: Some("power"),
        risk: Risk::Confirm,
    },
    Row {
        command: SystemCommand::Reload,
        id: "reload",
        title: "Reload",
        aliases: &["Reload Sill", "Refresh"],
        glyph: None,
        risk: Risk::Direct,
    },
];

const CONFIRM_ACTION: &str = "confirm";

/// Whether logind currently allows a command that can say no (`CanSuspend`/`CanReboot`/
/// `CanPowerOff`); every other command is always [`Offer::Available`] from this provider's own
/// side (whether it actually succeeds is the daemon's business at activation time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Offer {
    Available,
    Unavailable,
}

/// A snapshot of what logind allows, taken once and reused for the process's lifetime (a fresh
/// `zbus::Connection::system` per keystroke would blow the 50 ms budget; session policy does not
/// change keystroke to keystroke).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Availability {
    suspend: Offer,
    restart: Offer,
    shut_down: Offer,
}

impl Availability {
    /// Logind unreachable, or a property it could not answer: offer nothing that needs it
    /// (the "reports `Unavailable` cleanly" half of this wave's brief).
    const NONE: Availability = Availability {
        suspend: Offer::Unavailable,
        restart: Offer::Unavailable,
        shut_down: Offer::Unavailable,
    };

    fn of(self, command: SystemCommand) -> Offer {
        match command {
            SystemCommand::Suspend => self.suspend,
            SystemCommand::Restart => self.restart,
            SystemCommand::ShutDown => self.shut_down,
            SystemCommand::Lock | SystemCommand::LogOut | SystemCommand::Reload => Offer::Available,
        }
    }
}

/// Rows for the session commands whose names match the query.
#[derive(Debug, Default)]
pub struct SystemProvider {
    availability: OnceLock<Availability>,
}

impl SystemProvider {
    pub fn new() -> SystemProvider {
        SystemProvider::default()
    }

    /// A provider whose availability is fixed, never touching logind — for tests of the
    /// matching and activation logic, which have nothing to do with what this machine's session
    /// happens to allow right now.
    #[cfg(test)]
    fn with_availability(availability: Availability) -> SystemProvider {
        let cell = OnceLock::new();
        let _ = cell.set(availability);
        SystemProvider { availability: cell }
    }

    fn availability(&self) -> Availability {
        *self.availability.get_or_init(probe_availability)
    }
}

impl Provider for SystemProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::System
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let needle = Needle::parse(&query.text);
        let weights = RankWeights::DEFAULT;
        let mut matcher = matching::matcher();
        let availability = self.availability();

        let mut items = Vec::new();
        for row in ROWS {
            if availability.of(row.command) == Offer::Unavailable {
                continue;
            }
            let mut texts = Vec::with_capacity(row.aliases.len() + 1);
            texts.push(row.title);
            texts.extend_from_slice(row.aliases);
            let Some(score) = texts
                .iter()
                .filter_map(|text| {
                    rank_text(&needle, text, FrecencyScore::NONE, &weights, &mut matcher)
                })
                .max()
            else {
                continue;
            };
            items.push(Item {
                id: ItemId(row.id.to_owned()),
                provider: ProviderKind::System,
                title: row.title.to_owned(),
                subtitle: String::new(),
                icon: row
                    .glyph
                    .map_or(IconRef::None, |name| IconRef::Glyph(name.to_owned())),
                score,
                actions: confirm_action(row),
            });
        }

        if query.is_cancelled() {
            return;
        }
        items.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
        sink.push(Batch {
            provider: ProviderKind::System,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        let Some(row) = row_of(&item.id) else {
            return Activation::Nothing;
        };
        match (row.risk, choice) {
            (Risk::Direct, Choice::Primary) => Activation::System(row.command),
            (Risk::Confirm, Choice::Action(id)) if id.0 == CONFIRM_ACTION => {
                Activation::System(row.command)
            }
            (Risk::Confirm, Choice::Primary) | (_, Choice::Action(_)) => Activation::Nothing,
        }
    }
}

fn confirm_action(row: &Row) -> Vec<Action> {
    match row.risk {
        Risk::Direct => Vec::new(),
        Risk::Confirm => vec![Action {
            id: ActionId(CONFIRM_ACTION.to_owned()),
            label: format!("Confirm {}", row.title),
        }],
    }
}

fn row_of(id: &ItemId) -> Option<&'static Row> {
    ROWS.iter().find(|row| row.id == id.0)
}

/// `Yes` and `Challenge` (a polkit prompt) both mean logind lets the launcher offer the row; a
/// prompt the user then cancels is no different from cancelling any other launcher action.
fn offer_of(supported: IsSupported) -> Offer {
    match supported {
        IsSupported::Yes | IsSupported::Challenge => Offer::Available,
        IsSupported::No | IsSupported::NA => Offer::Unavailable,
    }
}

/// The real check, bridged from this trait's sync `query` into async zbus: `results::spawn`
/// (M4 launcher-core, plan "spawn_blocking per provider") runs every provider's `query` inside
/// `tokio::task::spawn_blocking`, and `Handle::block_on` from inside a blocking-pool task is the
/// documented way back into async code there — this panics only if called directly from an
/// async task instead (never done here; `tests/providers_system_live.rs` mirrors the real
/// call shape with its own `spawn_blocking`).
fn probe_availability() -> Availability {
    tokio::runtime::Handle::try_current()
        .ok()
        .and_then(|handle| handle.block_on(probe_logind()).ok())
        .unwrap_or(Availability::NONE)
}

/// `CanSuspend`/`CanReboot`/`CanPowerOff` only: read-only properties, never `Suspend`,
/// `Reboot`, `PowerOff` or `TerminateSession` themselves (those are the daemon's call, once it
/// receives the `Activation` this provider produced).
async fn probe_logind() -> zbus::Result<Availability> {
    let connection = zbus::Connection::system().await?;
    let manager = ManagerProxy::new(&connection).await?;
    Ok(Availability {
        suspend: offer_of(manager.can_suspend().await?),
        restart: offer_of(manager.can_reboot().await?),
        shut_down: offer_of(manager.can_power_off().await?),
    })
}

#[cfg(test)]
mod tests {
    use super::{Availability, Offer, SystemProvider};
    use sill_launcher::{
        ActionId, Cancel, Choice, Item, ItemId, Provider, ProviderKind, Query, Score,
        SystemCommand, VecSink,
    };

    const ALL_AVAILABLE: Availability = Availability {
        suspend: Offer::Available,
        restart: Offer::Available,
        shut_down: Offer::Available,
    };

    fn titles(provider: &SystemProvider, text: &str) -> Vec<String> {
        let cancel = Cancel::new();
        let query = Query::begin(text, &cancel);
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0
            .into_iter()
            .flat_map(|batch| batch.items)
            .map(|item| item.title)
            .collect()
    }

    #[test]
    fn a_query_table_matches_titles_and_aliases() {
        let provider = SystemProvider::with_availability(ALL_AVAILABLE);
        const CASES: &[(&str, &[&str])] = &[
            (
                "",
                &[
                    "Lock",
                    "Log Out",
                    "Reload",
                    "Restart",
                    "Shut Down",
                    "Suspend",
                ],
            ),
            ("lock", &["Lock"]),
            ("sign", &["Log Out"]),
            ("sleep", &["Suspend"]),
            ("reboot", &["Restart"]),
            ("power off", &["Shut Down"]),
            ("refresh", &["Reload"]),
            ("zzz", &[]),
        ];
        for &(text, want) in CASES {
            let mut got = titles(&provider, text);
            got.sort();
            let mut want = want.to_vec();
            want.sort_unstable();
            assert_eq!(got, want, "{text:?}");
        }
    }

    #[test]
    fn unavailable_commands_do_not_list() {
        let provider = SystemProvider::with_availability(Availability::NONE);
        let mut got = titles(&provider, "");
        got.sort();
        assert_eq!(got, vec!["Lock", "Log Out", "Reload"]);
    }

    #[test]
    fn a_cancelled_query_yields_nothing() {
        let provider = SystemProvider::with_availability(ALL_AVAILABLE);
        let cancel = Cancel::new();
        let query = Query::begin("lock", &cancel);
        cancel.begin();
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        assert!(sink.0.is_empty());
    }

    fn item(id: &str, title: &str) -> Item {
        Item {
            id: ItemId(id.to_owned()),
            provider: ProviderKind::System,
            title: title.to_owned(),
            subtitle: String::new(),
            icon: sill_launcher::IconRef::None,
            score: Score(0),
            actions: Vec::new(),
        }
    }

    #[test]
    fn a_direct_command_activates_on_enter() {
        let provider = SystemProvider::with_availability(ALL_AVAILABLE);
        assert_eq!(
            provider.activate(&item("lock", "Lock"), &Choice::Primary),
            sill_launcher::Activation::System(SystemCommand::Lock)
        );
    }

    #[test]
    fn a_destructive_command_needs_its_confirm_action() {
        let provider = SystemProvider::with_availability(ALL_AVAILABLE);
        let restart = item("restart", "Restart");
        assert_eq!(
            provider.activate(&restart, &Choice::Primary),
            sill_launcher::Activation::Nothing,
            "Enter alone must not restart the machine"
        );
        assert_eq!(
            provider.activate(&restart, &Choice::Action(ActionId("confirm".to_owned()))),
            sill_launcher::Activation::System(SystemCommand::Restart)
        );
    }

    #[test]
    fn an_unknown_row_id_activates_to_nothing() {
        let provider = SystemProvider::with_availability(ALL_AVAILABLE);
        assert_eq!(
            provider.activate(&item("no-such-row", "?"), &Choice::Primary),
            sill_launcher::Activation::Nothing
        );
    }
}
