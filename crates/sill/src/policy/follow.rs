//! [`follow`]: the one task that keeps a service told what its [`Policy`] answers.

use sill_services::Control;
use sill_settings::ShellFile;
use tokio::runtime::Handle;
use tokio::sync::watch;

use super::Policy;

/// Send `policy`'s answer for `settings` to `control` now and after every change that changes
/// it, on `rt`, until the settings, the policy's other source or the service is gone.
pub fn follow<P: Policy>(
    rt: &Handle,
    mut settings: watch::Receiver<ShellFile>,
    mut policy: P,
    control: Control<P::Command>,
) {
    rt.spawn(async move {
        let mut sent = None;
        loop {
            let out = policy.of(&settings.borrow_and_update());
            if sent.as_ref() != Some(&out) {
                if control.send(P::command(out.clone())).is_err() {
                    return;
                }
                sent = Some(out);
            }
            let changed = tokio::select! {
                changed = settings.changed() => changed,
                changed = policy.changed() => changed,
            };
            if changed.is_err() {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_services::Control;
    use sill_settings::ShellFile;
    use tokio::sync::mpsc::UnboundedReceiver;
    use tokio::sync::watch;

    use super::follow;
    use crate::policy::Policy;

    /// The dock's size as a number, a stand-in service's whole policy.
    struct DockSize;

    impl Policy for DockSize {
        type Command = u16;
        type Out = u16;

        fn of(&self, file: &ShellFile) -> u16 {
            file.dock.tile_size_px.0
        }

        fn command(out: u16) -> u16 {
            out
        }
    }

    fn next(rt: &tokio::runtime::Runtime, commands: &mut UnboundedReceiver<u16>) -> Option<u16> {
        rt.block_on(async {
            tokio::time::timeout(Duration::from_millis(300), commands.recv())
                .await
                .ok()
                .flatten()
        })
    }

    #[test]
    fn the_answer_is_sent_at_once_then_only_when_it_changes() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let (settings_tx, settings) = watch::channel(ShellFile::default());
        let (control, mut commands) = Control::channel();
        follow(rt.handle(), settings, DockSize, control);
        let first = ShellFile::default().dock.tile_size_px.0;
        assert_eq!(next(&rt, &mut commands), Some(first), "sent at start");

        let mut other = ShellFile::default();
        other.dock.badge_size_px.0 += 1;
        settings_tx.send_replace(other.clone());
        assert_eq!(
            next(&rt, &mut commands),
            None,
            "a change elsewhere sends nothing"
        );

        other.dock.tile_size_px.0 = first + 8;
        settings_tx.send_replace(other);
        assert_eq!(
            next(&rt, &mut commands),
            Some(first + 8),
            "a change sends again"
        );
    }
}
