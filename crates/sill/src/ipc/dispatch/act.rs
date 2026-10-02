//! `Act`: change something by telling a service or writing a file. Every arm answers `Done` once
//! the change is asked for; the change itself happens in the service or the settings watch.

use ds_settings::Store;
use sill_ipc::act::{ActRequest, DockRequest};
use sill_ipc::response::{FailureKind, Response};
use sill_model::clipboard::ClipboardCommand;
use sill_model::locker::LockerCommand;
use sill_model::notifications::NotificationsCommand;
use sill_model::pins::{PinAt, PinEvent};
use sill_model::screenshots::{ShotCommand, ShotKind};
use sill_settings::{DesktopWidgets, ShellFile};
use tokio::sync::watch;

use super::reach::Reach;
use super::reply::{failed, sent};
use super::{keys, switcher, workspace};

/// The change `request` asks for.
pub fn dispatch(reach: &Reach, request: ActRequest) -> Response {
    let services = reach.services;
    match request {
        ActRequest::Dock(request) => dock(reach, request),
        ActRequest::Display(request) => keys::press(reach, request),
        ActRequest::Lock => sent(services.locker.control.send(LockerCommand::Lock), "locker"),
        ActRequest::Screenshot(kind) => screenshot(reach, kind),
        ActRequest::ToggleDesktopWidgets => toggle_desktop_widgets(reach.store, reach.settings),
        ActRequest::Switcher(request) => switcher::chord(reach, request),
        ActRequest::Workspace(request) => workspace::step(reach, request),
        ActRequest::ClearClipboard => sent(
            services.clipboard.control.send(ClipboardCommand::Clear),
            "clipboard",
        ),
        ActRequest::DismissNotifications => sent(
            services
                .notifications
                .control
                .send(NotificationsCommand::DismissAll),
            "notifications",
        ),
        ActRequest::Keycap(command) => sent(services.keycap.control.send(command.into()), "keycap"),
        ActRequest::FocusLatest => {
            reach.links.banners.ask_keyboard();
            Response::Done
        }
    }
}

fn dock(reach: &Reach, request: DockRequest) -> Response {
    let event = match request {
        DockRequest::Pin { app } => PinEvent::Pin {
            app,
            at: PinAt::End,
        },
        DockRequest::Unpin { app } => PinEvent::Unpin(app),
    };
    sent(reach.services.pins.control.send(event), "pins")
}

fn screenshot(reach: &Reach, kind: ShotKind) -> Response {
    sent(
        reach
            .services
            .screenshots
            .control
            .send(ShotCommand::Take(kind)),
        "screenshots",
    )
}

/// Flip `widgets.desktop` in `settings.toml`, and publish the file at once so the layer follows
/// before the watch has fired. The file stays the one truth.
fn toggle_desktop_widgets(store: &Store, settings: &watch::Sender<ShellFile>) -> Response {
    let mut file = crate::daemon::settings::load(store);
    file.widgets.desktop = flipped(file.widgets.desktop);
    if let Err(error) = crate::daemon::settings::save(store, &file) {
        return failed(
            FailureKind::Internal,
            format!("saving the settings: {error}"),
        );
    }
    settings.send_replace(file);
    Response::Done
}

fn flipped(now: DesktopWidgets) -> DesktopWidgets {
    match now {
        DesktopWidgets::Off => DesktopWidgets::On,
        DesktopWidgets::On => DesktopWidgets::Off,
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::response::Response;
    use sill_settings::{DesktopWidgets, ShellFile};
    use tokio::sync::watch;

    use super::{flipped, toggle_desktop_widgets};

    #[test]
    fn toggling_flips_the_key() {
        assert_eq!(flipped(DesktopWidgets::Off), DesktopWidgets::On);
        assert_eq!(flipped(DesktopWidgets::On), DesktopWidgets::Off);
    }

    #[test]
    fn toggle_desktop_flips_saves_and_publishes() {
        let dir = std::env::temp_dir().join(format!("sill-widgets-toggle-{}", std::process::id()));
        let store = crate::daemon::settings::store(&dir);
        let (tx, rx) = watch::channel(ShellFile::default());
        for want in [DesktopWidgets::On, DesktopWidgets::Off] {
            assert_eq!(toggle_desktop_widgets(&store, &tx), Response::Done);
            assert_eq!(crate::daemon::settings::load(&store).widgets.desktop, want);
            assert_eq!(rx.borrow().widgets.desktop, want);
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
