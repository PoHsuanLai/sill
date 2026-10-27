//! The notifications service's policy, from the settings and the focus mode: sent once at start
//! (which ends the service's passive start) and again whenever either changes.

use sill_model::notifications::{
    HistoryCap, NotificationsCommand, NotificationsPolicy, Quiet, ServerMode,
};
use sill_settings::{FocusMode, NotificationsSettings, ShellFile};
use tokio::sync::watch;
use tokio::sync::watch::error::RecvError;

use super::Policy;

/// The most notifications the notification center keeps; the oldest go first.
const HISTORY_CAP: HistoryCap = HistoryCap(100);

/// The policy `n` and `focus` make: the service owns the bus name.
pub fn policy_of(n: &NotificationsSettings, focus: FocusMode) -> NotificationsPolicy {
    NotificationsPolicy {
        server: ServerMode::Own,
        quiet: match focus {
            FocusMode::Off => Quiet::Off,
            FocusMode::DoNotDisturb => Quiet::DoNotDisturb,
        },
        style: n.banner_style,
        history_cap: HISTORY_CAP,
        swipe: n.swipe,
    }
}

/// The notifications service's policy from the settings and `focus`. The focus mode comes from
/// the focus link rather than `notifications.dnd` in the file, so Do Not Disturb takes effect as
/// the control center sets it, before the file round-trips.
#[derive(Debug)]
pub struct NotificationsBridge {
    pub focus: watch::Receiver<FocusMode>,
}

impl Policy for NotificationsBridge {
    type Command = NotificationsCommand;
    type Out = NotificationsPolicy;

    fn of(&self, file: &ShellFile) -> NotificationsPolicy {
        policy_of(&file.notifications, *self.focus.borrow())
    }

    fn command(policy: NotificationsPolicy) -> NotificationsCommand {
        NotificationsCommand::Policy(policy)
    }

    fn changed(&mut self) -> impl Future<Output = Result<(), RecvError>> + Send {
        self.focus.changed()
    }
}

#[cfg(test)]
mod tests {
    use super::policy_of;
    use sill_model::notifications::Swipe;
    use sill_model::notifications::{HistoryCap, Presentation, Quiet, ServerMode};
    use sill_settings::{FocusMode, NotificationsSettings};

    #[test]
    fn the_settings_and_focus_become_the_policy() {
        let defaults = policy_of(&NotificationsSettings::default(), FocusMode::Off);
        assert_eq!(defaults.server, ServerMode::Own, "own by default");
        assert_eq!(defaults.quiet, Quiet::Off);
        assert_eq!(defaults.style, Presentation::Banner);
        assert_eq!(defaults.history_cap, HistoryCap(100));
        assert_eq!(
            defaults.swipe,
            Swipe::KeepInCenter,
            "a swipe keeps it, as macOS"
        );
        let n = NotificationsSettings {
            banner_style: Presentation::HistoryOnly,
            swipe: Swipe::Dismiss,
            ..NotificationsSettings::default()
        };
        let quiet = policy_of(&n, FocusMode::DoNotDisturb);
        assert_eq!(quiet.quiet, Quiet::DoNotDisturb);
        assert_eq!(quiet.style, Presentation::HistoryOnly);
        assert_eq!(quiet.swipe, Swipe::Dismiss);
    }

    #[test]
    fn the_policy_is_sent_at_once_and_again_on_a_change() {
        use sill_model::notifications::NotificationsCommand;
        use sill_services::Control;
        use sill_settings::ShellFile;
        use tokio::sync::watch;

        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let (settings_tx, settings) = watch::channel(ShellFile::default());
        let (focus_tx, focus) = watch::channel(FocusMode::Off);
        let (control, mut commands) = Control::channel();
        crate::policy::follow(
            rt.handle(),
            settings,
            super::NotificationsBridge { focus },
            control,
        );
        let next = |commands: &mut tokio::sync::mpsc::UnboundedReceiver<NotificationsCommand>| {
            rt.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(2), commands.recv())
                    .await
                    .ok()
                    .flatten()
            })
        };
        let first = next(&mut commands);
        assert!(
            matches!(&first, Some(NotificationsCommand::Policy(p)) if p.quiet == Quiet::Off),
            "{first:?}"
        );
        let _ = focus_tx.send(FocusMode::DoNotDisturb);
        let second = next(&mut commands);
        assert!(
            matches!(&second, Some(NotificationsCommand::Policy(p)) if p.quiet == Quiet::DoNotDisturb),
            "{second:?}"
        );
        drop(settings_tx);
    }
}
