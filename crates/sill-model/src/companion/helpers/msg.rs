//! What tells the helpers screen things, and what it asks of the outside.

use std::collections::BTreeSet;

use companion_wire::{HelperCardView, HelperForm, HelperReply, HelpersView};
use docket_core::helper::{Ceiling, HelperModel};
use prov::{AppName, HelperId};

/// Which call a reply answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// Making a helper or changing one.
    Save,
    /// Giving a helper a job.
    Job,
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
    /// The person chose a model in the picker.
    ChooseModel(HelperModel),
    /// The person asks to choose a model.
    PickModel,
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
    /// Open Settings at the model picker.
    OpenModelPicker,
}
