//! Session commands: the power actions (`SessionAction::ALL`) and sill's own Reload.
//!
//! Every action logind offers is a row named and searched by `sill_model::session::words` (its
//! title and its aliases), the same words the bar's system menu and the power menu show; what
//! logind offers is the session service's state, read here and asked of logind nowhere else.
//! `activate` stays pure (`sill_launcher::Provider`'s own contract): it only says which
//! [`SystemCommand`] was chosen, and the launcher carries it out by `SessionAction::confirm`
//! (Lock and Sleep at once, Log Out, Restart and Shut Down through the power menu's question).

use sill_launcher::{
    Activation, Choice, FrecencyScore, IconRef, Item, ItemId, Needle, Provider, ProviderKind,
    Query, RankWeights, ResultSink, SessionAction, SystemCommand, matching, rank_text,
};
use sill_model::session::{Offer, Session, words};
use tokio::sync::watch;

/// The Reload row's words: sill's own command, not a power action.
const RELOAD_TITLE: &str = "Reload";
const RELOAD_ALIASES: &[&str] = &["Reload Sill", "Refresh"];

/// One row's shape: its command, its id (the frecency key, kept from before the actions had one
/// list), its title and search words, and its glyph (design/08-ICONS.md §1.6 has only `lock` and
/// `power`; Reload shows the provider's own glyph).
struct Row {
    command: SystemCommand,
    id: &'static str,
    title: &'static str,
    aliases: &'static [&'static str],
    glyph: Option<&'static str>,
}

/// The row of `action`.
fn action_row(action: SessionAction) -> Row {
    Row {
        command: SystemCommand::Session(action),
        id: action_id(action),
        title: words::title(action),
        aliases: words::aliases(action),
        glyph: Some(match action {
            SessionAction::Lock => "lock",
            SessionAction::LogOut
            | SessionAction::Restart
            | SessionAction::PowerOff
            | SessionAction::Suspend => "power",
        }),
    }
}

/// The Reload row.
fn reload_row() -> Row {
    Row {
        command: SystemCommand::Reload,
        id: "reload",
        title: RELOAD_TITLE,
        aliases: RELOAD_ALIASES,
        glyph: None,
    }
}

/// An action's row id.
fn action_id(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "lock",
        SessionAction::LogOut => "log_out",
        SessionAction::Restart => "restart",
        SessionAction::PowerOff => "shut_down",
        SessionAction::Suspend => "suspend",
    }
}

/// The rows `session` allows: every action logind offers (asking for a password first counts),
/// then Reload, which needs nothing of logind.
fn rows(session: &Session) -> Vec<Row> {
    SessionAction::ALL
        .into_iter()
        .filter(|&action| matches!(session.offer(action), Offer::Available | Offer::NeedsAuth))
        .map(action_row)
        .chain(std::iter::once(reload_row()))
        .collect()
}

/// The row named `id`, whatever logind offers now (a row already shown is still chosen).
fn row_of(id: &ItemId) -> Option<Row> {
    SessionAction::ALL
        .into_iter()
        .map(action_row)
        .chain(std::iter::once(reload_row()))
        .find(|row| row.id == id.0)
}

/// Rows for the session commands whose names match the query.
#[derive(Debug)]
pub struct SystemProvider {
    /// What logind offers: the session service's state.
    session: watch::Receiver<Session>,
}

impl SystemProvider {
    /// Rows for what `session` says logind offers.
    pub fn new(session: watch::Receiver<Session>) -> SystemProvider {
        SystemProvider { session }
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
        let session = *self.session.borrow();
        let mut items: Vec<Item> = rows(&session)
            .into_iter()
            .filter_map(|row| {
                let score = std::iter::once(row.title)
                    .chain(row.aliases.iter().copied())
                    .filter_map(|text| {
                        rank_text(&needle, text, FrecencyScore::NONE, &weights, &mut matcher)
                    })
                    .max()?;
                Some(Item {
                    id: ItemId(row.id.to_owned()),
                    provider: ProviderKind::System,
                    title: row.title.to_owned(),
                    subtitle: String::new(),
                    icon: row
                        .glyph
                        .map_or(IconRef::None, |name| IconRef::Glyph(name.to_owned())),
                    score,
                    actions: Vec::new(),
                    subject: sill_launcher::Subject::None,
                })
            })
            .collect();
        if query.is_cancelled() {
            return;
        }
        items.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
        sink.push(items);
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (row_of(&item.id), choice) {
            (Some(row), Choice::Primary) => Activation::System(row.command),
            (Some(_), Choice::Action(_)) | (None, _) => Activation::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SystemProvider;
    use sill_launcher::{
        ActionId, Activation, Cancel, Choice, Item, ItemId, Provider, ProviderKind, Query, Score,
        SessionAction, SystemCommand, VecSink,
    };
    use sill_model::session::{Offer, Session};
    use sill_model::units::Presence;
    use tokio::sync::watch;

    fn provider(session: Session) -> SystemProvider {
        let (_tx, rx) = watch::channel(session);
        SystemProvider::new(rx)
    }

    fn titles(provider: &SystemProvider, text: &str) -> Vec<String> {
        let cancel = Cancel::new();
        let query = Query::begin(text, &cancel);
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        let mut got: Vec<String> = sink.0.into_iter().flatten().map(|i| i.title).collect();
        got.sort();
        got
    }

    #[test]
    fn a_query_table_matches_titles_and_aliases() {
        let provider = provider(Session::everything());
        const CASES: &[(&str, &[&str])] = &[
            (
                "",
                &[
                    "Lock Screen",
                    "Log Out",
                    "Reload",
                    "Restart",
                    "Shut Down",
                    "Sleep",
                ],
            ),
            ("lock", &["Lock Screen"]),
            ("logout", &["Log Out"]),
            ("log out", &["Log Out"]),
            ("sign", &["Log Out"]),
            ("shutdown", &["Shut Down"]),
            ("shut", &["Shut Down"]),
            ("power off", &["Shut Down"]),
            ("sleep", &["Sleep"]),
            ("suspend", &["Sleep"]),
            ("reboot", &["Restart"]),
            ("refresh", &["Reload"]),
            ("zzz", &[]),
        ];
        for &(text, want) in CASES {
            let mut want = want.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            want.sort();
            assert_eq!(titles(&provider, text), want, "{text:?}");
        }
    }

    #[test]
    fn only_what_the_session_offers_lists() {
        let offline = Session::default();
        assert_eq!(
            titles(&provider(offline), ""),
            vec!["Reload"],
            "logind never answered"
        );
        let no_power = Session {
            presence: Presence::Present,
            lock: Offer::Available,
            log_out: Offer::Available,
            restart: Offer::NeedsAuth,
            ..Session::default()
        };
        assert_eq!(
            titles(&provider(no_power), ""),
            vec!["Lock Screen", "Log Out", "Reload", "Restart"],
            "a password prompt still offers it"
        );
    }

    #[test]
    fn the_list_follows_the_session_service() {
        let (tx, rx) = watch::channel(Session::default());
        let provider = SystemProvider::new(rx);
        assert_eq!(titles(&provider, "log"), Vec::<String>::new());
        tx.send_replace(Session::everything());
        assert_eq!(titles(&provider, "log"), vec!["Log Out"]);
    }

    #[test]
    fn a_cancelled_query_yields_nothing() {
        let provider = provider(Session::everything());
        let cancel = Cancel::new();
        let query = Query::begin("lock", &cancel);
        cancel.begin();
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        assert!(sink.0.is_empty());
    }

    fn item(id: &str) -> Item {
        Item {
            id: ItemId(id.to_owned()),
            provider: ProviderKind::System,
            title: id.to_owned(),
            subtitle: String::new(),
            icon: sill_launcher::IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: sill_launcher::Subject::None,
        }
    }

    /// Enter on a row names its command (the launcher asks first by `SessionAction::confirm`);
    /// no row carries an action, and an unknown id is nothing.
    #[test]
    fn every_row_names_its_command_on_enter() {
        let provider = provider(Session::everything());
        let cases = [
            ("lock", SystemCommand::Session(SessionAction::Lock)),
            ("log_out", SystemCommand::Session(SessionAction::LogOut)),
            ("restart", SystemCommand::Session(SessionAction::Restart)),
            ("shut_down", SystemCommand::Session(SessionAction::PowerOff)),
            ("suspend", SystemCommand::Session(SessionAction::Suspend)),
            ("reload", SystemCommand::Reload),
        ];
        for (id, command) in cases {
            assert_eq!(
                provider.activate(&item(id), &Choice::Primary),
                Activation::System(command),
                "{id}"
            );
            assert_eq!(
                provider.activate(&item(id), &Choice::Action(ActionId("confirm".to_owned()))),
                Activation::Nothing,
                "{id}: no row action"
            );
        }
        assert_eq!(
            provider.activate(&item("no-such-row"), &Choice::Primary),
            Activation::Nothing
        );
    }
}
