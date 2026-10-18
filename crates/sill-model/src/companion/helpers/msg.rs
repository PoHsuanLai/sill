//! What tells the helpers screen things, and what it asks of the outside.

use std::collections::BTreeSet;

use companion_wire::{HelperCardView, HelperForm, HelperReply, HelpersView};
use docket_core::helper::{Ceiling, HelperModel, TerminalRequest};
use porter_infer::ModelRef;
use prov::{AppName, HelperId};

use super::automatic::PoolWritten;
use super::models::ModelGroup;
use super::routines::{RoutinesCommand, RoutinesMsg};

/// Which call a reply answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// Making a helper or changing one.
    Save,
    /// Giving a helper a job.
    Job,
    /// Signing in to the app a helper runs.
    SignIn,
    /// Signing out of the app a helper runs.
    SignOut,
    /// Opening a helper's work in a terminal.
    OpenInTerminal,
    /// Bringing a helper's work back from a terminal, as the person asked.
    BringBack,
    /// Bringing a helper's work back because its terminal window was closed: if the work is
    /// already back, there is nothing to say.
    BringBackAfterClose,
}

/// What the screen is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpersMsg {
    /// The list arrived.
    Listed(HelpersView),
    /// The companion says something about the helpers changed: read the list again.
    Changed,
    /// A card arrived.
    CardRead(Box<HelperCardView>),
    /// The companion has no such helper.
    CardMissing(HelperId),
    /// The apps a helper may be given, as the installed apps declare them.
    AppsOffered(BTreeSet<AppName>),
    /// A call's answer.
    Replied(Asked, HelperReply),
    /// A call could not be made at all.
    Unreached,
    /// The person opens a helper's card.
    OpenCard(HelperId),
    /// The person closes the card area.
    CloseCard,
    /// The person starts a new helper.
    NewHelper,
    /// The person types a name.
    EditName(String),
    /// The person types what it is for.
    EditRole(String),
    /// The person ticks or unticks an app.
    ToggleApp(AppName),
    /// The person chooses how far it may go.
    SetCeiling(Ceiling),
    /// The person chose a model from the card's list.
    ChooseModel(HelperModel),
    /// The person asks to choose a model: the card lists the models there are, or hides the list.
    PickModel,
    /// The models there are arrived, grouped by where they run.
    ModelsOffered(Vec<ModelGroup>),
    /// The person asks to add a model: Settings opens.
    AddModel,
    /// The person saves the card.
    Save,
    /// The person pauses a helper.
    Pause(HelperId),
    /// The person resumes a helper, or asks one whose job did not finish to try again.
    Resume(HelperId),
    /// The person asks to remove a helper; they are asked to confirm.
    AskRemove(HelperId),
    /// The person decides not to remove it.
    CancelRemove,
    /// The person confirms removing a helper they were asked about.
    Remove(HelperId),
    /// The person gives the open helper a job.
    GiveJob(String),
    /// The person signs in to the app a helper runs, by one of the ways its card lists.
    SignIn(HelperId, String),
    /// The person signs out of the app a helper runs.
    SignOut(HelperId),
    /// The window the sign-in opened was closed: the companion checks again.
    TerminalClosed(HelperId),
    /// The terminal a helper's work was handed to was closed: the work is asked for back.
    HandoffClosed(HelperId),
    /// A terminal window could not be opened.
    NoTerminal,
    /// A terminal request came without the app's command inside its sandbox, so nothing was
    /// opened: the app never runs unconfined.
    SignInUnconfined,
    /// The person hands a helper's work to a terminal.
    OpenInTerminal(HelperId),
    /// The person asks for a helper's work back from a terminal.
    BringBack(HelperId),
    /// The person chooses Automatic for the open helper: they tick the models it may use.
    StartAutomatic,
    /// The person ticks or unticks a model for Automatic.
    TogglePooled(ModelRef),
    /// The person gives up choosing the models Automatic may use.
    CancelAutomatic,
    /// The person confirms the models ticked for Automatic.
    ConfirmAutomatic,
    /// What became of keeping the models Automatic may use.
    PoolWritten(PoolWritten),
    /// Something about the routines.
    Routines(RoutinesMsg),
}

/// What the screen asks the companion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpersCommand {
    /// Read the list, with what the helpers told the person after this number.
    List {
        /// The number of the newest sentence seen.
        after: u64,
    },
    /// Read a helper's card.
    ReadCard(HelperId),
    /// Make a helper.
    Create(Box<HelperForm>),
    /// Change a helper.
    Edit {
        /// Which.
        helper: HelperId,
        /// What it becomes.
        form: Box<HelperForm>,
    },
    /// Pause a helper.
    Pause(HelperId),
    /// Resume a helper.
    Resume(HelperId),
    /// Remove a helper.
    Remove(HelperId),
    /// Give a helper a job.
    PostJob {
        /// Which.
        helper: HelperId,
        /// What it is to do, in the person's words.
        goal: String,
    },
    /// Read the models a helper may be given.
    ReadModels,
    /// Open Settings at the model picker.
    OpenModelPicker,
    /// Sign in to the app a helper runs, by one of the ways its card lists.
    SignIn {
        /// Which.
        helper: HelperId,
        /// The id of the way.
        way: String,
    },
    /// Sign out of the app a helper runs.
    SignOut(HelperId),
    /// Open a terminal window running the app, as the companion asked; when it closes the screen is
    /// told ([`HelpersMsg::TerminalClosed`], or [`HelpersMsg::HandoffClosed`] for a handover).
    OpenTerminal(TerminalRequest),
    /// Ask the companion to hand a helper's work to a terminal.
    OpenInTerminal(HelperId),
    /// Ask the companion to take a helper's work back from a terminal.
    BringBack(HelperId),
    /// Ask the companion for a helper's work back because its terminal window was closed.
    BringBackAfterClose(HelperId),
    /// Keep the models Automatic may use for one helper.
    SetPool {
        /// Which.
        helper: HelperId,
        /// The models, never empty.
        pool: BTreeSet<ModelRef>,
    },
    /// Something about the routines.
    Routines(RoutinesCommand),
}
