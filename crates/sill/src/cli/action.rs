//! `sill do ACTION`: any action of the shell by its id, as a menu or a chord asks for it. The
//! other verbs are the same asks under friendlier words; this one needs no verb of its own when
//! an action is added. What the action is done to is named by at most one of `--app`,
//! `--windows` and `--settings`; the shell refuses a subject the action cannot use.

use clap::Args;
use sill_ipc::request::Request;
use sill_launcher::actions::ActionId;
use sill_launcher::{AppId, SettingsTarget, WindowKey};
use sill_model::actions::{Ask, Subject};

use super::word::word;

/// `sill do ...`.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct DoArgs {
    /// The action's id, for example `lock`, `next-space` or `quit-app`.
    #[arg(value_parser = word::<ActionId>())]
    pub action: ActionId,
    #[command(flatten)]
    pub on: DoSubject,
}

/// What the action is done to: nothing, or one of these.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[group(multiple = false)]
pub struct DoSubject {
    /// An app, by desktop id (`org.mozilla.firefox`): open-app, pin-app, force-quit-app.
    #[arg(long, value_name = "APP")]
    pub app: Option<String>,
    /// Windows, by the keys the shell lists them with: hide-app, quit-app.
    #[arg(long, value_name = "KEYS", value_delimiter = ',')]
    pub windows: Option<Vec<u64>>,
    /// A place in Settings, for example `page:wallpaper`: open-settings.
    #[arg(long, value_name = "PLACE")]
    pub settings: Option<SettingsTarget>,
}

impl DoSubject {
    /// What the flags name.
    fn subject(&self) -> Subject {
        match (&self.app, &self.windows, &self.settings) {
            (Some(app), _, _) => Subject::App(AppId(app.clone())),
            (None, Some(keys), _) => {
                Subject::Windows(keys.iter().copied().map(WindowKey).collect())
            }
            (None, None, Some(place)) => Subject::Settings(place.clone()),
            (None, None, None) => Subject::None,
        }
    }
}

/// The request `sill do ACTION ...` sends.
pub fn do_action_request(args: &DoArgs) -> Request {
    Request::Do(Ask::on(args.action, args.on.subject()))
}
