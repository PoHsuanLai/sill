//! The sequence itself, over a host it can drive with fakes.

use std::time::Duration;

use sill_launcher::WindowKey;

/// How long the apps get to close their windows before the session stays as it is.
pub(crate) const QUIT_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the open windows are looked at while waiting.
pub(crate) const POLL: Duration = Duration::from_millis(250);

/// One app window that is still open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenWindow {
    pub(crate) key: WindowKey,
    pub(crate) app_id: String,
}

/// How the sequence ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuitOutcome {
    /// No window is left and the apps without one were asked to stop: the session may end.
    Quit,
    /// These windows stayed open past the timeout: the session must not end.
    Held(Vec<OpenWindow>),
}

/// What the sequence reaches: the compositor's windows and the user's systemd manager.
pub(crate) trait AppsHost {
    /// Every app window open now.
    fn open_windows(&self) -> Vec<OpenWindow>;
    /// Ask one window to close.
    fn close(&self, key: WindowKey);
    /// Let `span` pass.
    async fn wait(&self, span: Duration);
    /// Ask the apps that run without a window to stop.
    async fn stop_scopes(&self);
}

/// Close every window, wait until none is left or [`QUIT_TIMEOUT`] has passed, then stop the
/// windowless apps. Time is counted in [`POLL`] steps through [`AppsHost::wait`].
pub(crate) async fn quit_apps<H: AppsHost>(host: &H) -> QuitOutcome {
    for window in host.open_windows() {
        host.close(window.key);
    }
    let mut waited = Duration::ZERO;
    loop {
        let left = host.open_windows();
        if left.is_empty() {
            host.stop_scopes().await;
            return QuitOutcome::Quit;
        }
        if waited >= QUIT_TIMEOUT {
            return QuitOutcome::Held(left);
        }
        host.wait(POLL).await;
        waited += POLL;
    }
}
