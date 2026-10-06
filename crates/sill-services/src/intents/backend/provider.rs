//! sill as an `IntentProvider1`: the shell's own actions. intentd calls `Perform`; the provider
//! reads the invocation into a [`ShellAction`], has the daemon do it (a [`ShellCall`]) and says
//! how it went. Everything else a provider can be asked is answered with nothing to say: the
//! shell indexes nothing, previews nothing and declares no `dry_run`.

use std::time::Duration;

use docket_client::{ContextSource, DbusTransport, IntentProvider, SummonTarget};
use docket_core::{
    AppRefusal, ContextScope, ContextSnapshot, EntityRef, FailText, Here, Hit, Invocation,
    Manifest, Outcome, Preview, Selection, SuggestAsk, SummonAnswer, SummonOrigin, SummonSerial,
    TextTarget, UndoFault, UndoToken, ValidManifest, Visible, WindowPrivacy, validate,
};
use porter_core::Count;
use prov::{Actor, EntityId, Label, Labelled};
use sill_model::intents::shell::{
    ShellAction, ShellFault, action_of, menu_ask_of, outcome_of, undo_of, window_ask_of,
};
use sill_model::screenshots::Shots;
use tokio::sync::{mpsc, oneshot, watch};

use crate::app_menu::{MenuAgent, MenuLinks};
use crate::intents::ShellCall;

/// What the shell declares, as shipped: the installed file, held to the model by a test.
const MANIFEST: &str = include_str!("../../../../../dist/intents/org.quire.Shell.toml");

/// How long the daemon has to do one action (the manifest's `quick` latency is five seconds).
const CALL_WAIT: Duration = Duration::from_secs(4);

/// The shell's manifest, parsed and validated; an error is a bug in the shipped file.
pub fn manifest() -> Result<ValidManifest, String> {
    let parsed: Manifest = toml::from_str(MANIFEST).map_err(|e| e.to_string())?;
    validate(parsed).map_err(|e| e.to_string())
}

/// The shell's provider.
pub(super) struct ShellProvider {
    pub manifest: ValidManifest,
    pub calls: mpsc::UnboundedSender<ShellCall>,
    /// The focused window's menus, for `shell.menu.list` and `shell.menu.activate`.
    pub menus: MenuAgent<DbusTransport>,
    /// The windows and the compositor, for the window and workspace actions.
    pub windows: MenuLinks,
    /// The screenshots the shell has seen, for the path `shell.screenshot` answers with.
    pub shots: watch::Receiver<Shots>,
}

impl ShellProvider {
    /// Have the daemon do `action` and wait for how it went.
    async fn done(&self, action: ShellAction) -> Result<(), ShellFault> {
        let (reply, answer) = oneshot::channel();
        self.calls
            .send(ShellCall { action, reply })
            .map_err(|_| ShellFault::Failed("the shell is shutting down".to_owned()))?;
        match tokio::time::timeout(CALL_WAIT, answer).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ShellFault::Failed("the shell did not answer".to_owned())),
            Err(_) => Err(ShellFault::Failed("the shell took too long".to_owned())),
        }
    }
}

fn refusal(fault: ShellFault) -> AppRefusal {
    match fault {
        ShellFault::Unsupported => AppRefusal::Unsupported,
        ShellFault::Failed(why) => AppRefusal::Failed(FailText(why)),
    }
}

impl IntentProvider for ShellProvider {
    fn manifest(&self) -> &ValidManifest {
        &self.manifest
    }

    async fn perform(&self, inv: Invocation) -> Result<Outcome, AppRefusal> {
        if let Some(ask) = menu_ask_of(&inv)? {
            return super::menus::perform(&self.menus, ask, &inv).await;
        }
        if let Some(ask) = window_ask_of(&inv)? {
            return super::windows::perform(&self.windows, &ask, &inv);
        }
        let action = action_of(&inv)?;
        let first = self.shots.borrow().last_id;
        self.done(action.clone()).await.map_err(refusal)?;
        let mut outcome = outcome_of(&action);
        if let ShellAction::Screenshot(_) = action {
            outcome.value = super::shots::saved(&mut self.shots.clone(), first, &inv).await;
        }
        Ok(outcome)
    }

    async fn dry_run(&self, inv: Invocation) -> Result<Preview, AppRefusal> {
        if menu_ask_of(&inv)?.is_some() || window_ask_of(&inv)?.is_some() {
            return Ok(Preview::None);
        }
        action_of(&inv).map(|_| Preview::None)
    }

    async fn undo(&self, token: UndoToken, actor: Actor) -> Result<(), UndoFault> {
        let _ = actor;
        let action = undo_of(&token)?;
        self.done(action).await.map_err(|_| UndoFault::Gone)
    }

    async fn search(&self, text: &str) -> Vec<Hit> {
        let _ = text;
        Vec::new()
    }

    async fn preview(&self, id: &EntityId) -> Preview {
        let _ = id;
        Preview::None
    }

    async fn suggest(&self, ask: SuggestAsk) -> Vec<EntityRef> {
        let _ = ask;
        Vec::new()
    }
}

/// What the shell shows the router when asked "what is the person looking at": nothing in
/// particular, in the shell.
pub(super) struct ShellContext {
    pub app: prov::AppName,
}

impl ContextSource for ShellContext {
    fn snapshot(&self, scope: ContextScope) -> ContextSnapshot {
        let _ = scope;
        ContextSnapshot {
            app: self.app.clone(),
            window: Labelled {
                value: "sill".to_owned(),
                label: Label::trusted_user(),
            },
            here: Here::Nowhere,
            selection: Selection::Nothing,
            visible: Visible {
                kind: None,
                items: Vec::new(),
                total: Count(0),
            },
            text_target: TextTarget::None,
            privacy: WindowPrivacy::Normal,
        }
    }
}

/// The shell has no prompt field to summon the companion into.
pub(super) struct NoSummon;

impl SummonTarget for NoSummon {
    fn summon(&self, serial: SummonSerial, origin: SummonOrigin) -> SummonAnswer {
        let _ = (serial, origin);
        SummonAnswer::Declined
    }
}
