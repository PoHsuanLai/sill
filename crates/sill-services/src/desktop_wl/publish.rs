//! What the windows thread publishes: the projected state, whole, after every dispatch, and the
//! desktop's view projected from the same state.

use sill_model::desktop::DesktopView;
use sill_model::windows::WindowModel;
use tokio::sync::watch;

/// The sending halves of the service's state and of the desktop's view. Receivers wake only when
/// a step changed something.
#[derive(Debug)]
pub(crate) struct Publish {
    tx: watch::Sender<WindowModel>,
    desktop: watch::Sender<DesktopView>,
}

impl Publish {
    pub(crate) fn new(
        tx: watch::Sender<WindowModel>,
        desktop: watch::Sender<DesktopView>,
    ) -> Publish {
        Publish { tx, desktop }
    }

    /// Publish `next` and the `desktop` projected beside it, whole, as the arms do after every
    /// dispatch: they project the full state, so there is no event to step.
    pub(crate) fn replace(&self, next: WindowModel, desktop: DesktopView) {
        self.tx
            .send_if_modified(|now| replace_if_changed(now, |_| next));
        self.desktop
            .send_if_modified(|now| replace_if_changed(now, |_| desktop));
    }

    /// Back to empty: the compositor has gone, and its windows, workspaces and Spaces with it.
    pub(crate) fn clear(&self) {
        self.replace(WindowModel::default(), DesktopView::default());
    }
}

/// Put `step(now)` into `now`; whether that changed it (the `send_if_modified` contract).
fn replace_if_changed<T: Clone + PartialEq>(now: &mut T, step: impl FnOnce(T) -> T) -> bool {
    let next = step(now.clone());
    let changed = next != *now;
    *now = next;
    changed
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{ActiveView, DesktopView};
    use sill_model::windows::WindowModel;
    use tokio::sync::watch;

    use super::Publish;

    /// The state and the view go out together; a repeat wakes no one, and clearing empties both.
    #[test]
    fn the_state_and_the_view_are_published_together_and_a_repeat_wakes_no_one() {
        let (model_tx, mut model) = watch::channel(WindowModel::default());
        let (desktop_tx, mut desktop) = watch::channel(DesktopView::default());
        let publish = Publish::new(model_tx, desktop_tx);
        let view = DesktopView {
            active: ActiveView {
                app: Some("term".to_owned()),
                key_window: Some("id-1".to_owned()),
            },
            ..DesktopView::default()
        };

        publish.replace(WindowModel::default(), view.clone());
        assert_eq!(desktop.has_changed().ok(), Some(true));
        assert_eq!(*desktop.borrow_and_update(), view);
        assert_eq!(
            model.has_changed().ok(),
            Some(false),
            "the state was the same"
        );
        drop(model.borrow_and_update());

        publish.replace(WindowModel::default(), view);
        assert_eq!(desktop.has_changed().ok(), Some(false));

        publish.clear();
        assert_eq!(*desktop.borrow(), DesktopView::default());
    }
}
