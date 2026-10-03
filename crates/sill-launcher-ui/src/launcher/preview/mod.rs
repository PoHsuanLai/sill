//! The preview pane: Quick Look for the selected row, a
//! column right of the results inside the launcher card. What it shows is
//! `sill_launcher::preview_for(row)`; its keys are [`keys::step_preview`]; its moments go
//! through quire's `ds::motion::detail` in [`moment::use_pane_detail`].
//!
//! It is quire's `PreviewPane`, set beside the results as the palette's
//! `aside`: sill maps a [`Preview`] to quire's `PaneContent` (`content.rs`) and a row's actions
//! to `PaneAction`s with their chords. A file's content is the viewer's own pane
//! (`anyview_peek::Pane`) in `PaneContent::Slot`: the viewer's parts sniff and peek at it off the
//! UI thread (`load.rs`), so a file looks the same here as in the viewer's window, and sill
//! names no format. The pane never takes the keyboard: `focused` draws the ring on the action
//! the panel's Tab reached. The pane is the future Quick Look app's shared part (design/20 §2.4).

mod content;
pub mod keys;
pub mod load;
pub mod moment;

use dioxus::prelude::*;
use ds::components::lists::preview::pane::PaneAction;
use ds::motion::detail::touch::Touch;
use ds::motion::detail::use_operation::use_operation;
use ds::prelude::*;
use sill_launcher::{Action, Preview};

use crate::launcher::chord_keys::action_shortcut;
use crate::launcher::preview::content::Load;
use crate::launcher::preview::moment::{PaneState, use_pane_detail};

/// The pane's element id (for the blur region, if the card's does not cover it).
pub const PREVIEW_ID: &str = "launcher-preview";

/// The pane's width beside the 680 px card (quire's `ASIDE_WIDTH`; Spotlight's is about 45 % of
/// its window). The panel widens to fit it while the pane is shown (`layer::PANEL_ROOM_WIDTH`).
pub const PREVIEW_WIDTH_PX: u32 = 360;

/// The pane for `preview`, with the selected row's `actions` as its controls (their chords
/// shown, quire design/09 H5), `focused` the control with the keyboard (Tab order). The
/// launcher's panel draws [`LauncherPane`], which also takes the row's icon, the pane's showing
/// and what a click on a control does.
#[component]
pub fn PreviewPane(preview: Preview, actions: Vec<Action>, focused: Option<usize>) -> Element {
    rsx! {
        LauncherPane { preview, actions, focused }
    }
}

/// [`PreviewPane`] as the panel draws it: `icon` is the row's (an app's), `shown` plays the
/// pane in and out (quire calls `on_hidden` when it has left), `onaction` hears a click on a
/// control, `touch` is who caused this render's change (design/26 R5).
#[component]
pub fn LauncherPane(
    preview: Preview,
    actions: Vec<Action>,
    focused: Option<usize>,
    #[props(default)] icon: Option<IconSource>,
    #[props(default = Shown::Visible)] shown: Shown,
    #[props(default)] on_hidden: EventHandler<()>,
    #[props(default)] onaction: EventHandler<usize>,
    #[props(default)] touch: Touch,
) -> Element {
    let (content, load) = content::use_content(&preview, icon);
    // quire plays what the cue names: the entrance springs only on contact, another
    // kind or a load landing cross-fades the media, a slow load shows its pending look. One
    // `ds::prelude::PreviewPane` stays mounted whatever it shows, so a kind change is never an entrance.
    let pane = use_pane_detail(pane_state(shown, preview.kind(), load), touch);
    let operation = use_operation(pane.cue());
    let actions = pane_actions(&actions);
    rsx! {
        div { id: PREVIEW_ID, class: "sill-launcher-preview",
            ds::prelude::PreviewPane {
                content,
                actions,
                focused,
                onaction,
                shown,
                on_hidden,
                cue: pane.cue(),
                operation,
            }
        }
    }
}

/// The pane as the person sees it: leaving is hidden, then how far its content has loaded.
fn pane_state(shown: Shown, kind: sill_launcher::PreviewKind, load: Load) -> PaneState {
    match (shown, load) {
        (Shown::Hidden, Load::Ready | Load::Reading | Load::Failed) => PaneState::Hidden,
        (Shown::Visible, Load::Ready) => PaneState::Showing(kind),
        (Shown::Visible, Load::Reading) => PaneState::Pending,
        (Shown::Visible, Load::Failed) => PaneState::Failed,
    }
}

/// A row's actions as the pane's, each with its chord (none: no keys drawn).
pub fn pane_actions(actions: &[Action]) -> Vec<PaneAction> {
    actions
        .iter()
        .map(|action| PaneAction {
            label: action.label.clone(),
            shortcut: action_shortcut(&action.keys).unwrap_or(Shortcut(Vec::new())),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::content::Load;
    use super::moment::PaneState;
    use super::pane_state;
    use ds::prelude::*;
    use sill_launcher::PreviewKind;

    #[test]
    fn the_pane_state_follows_its_showing_and_its_load() {
        let file = PreviewKind::File;
        let cases = [
            (Shown::Visible, Load::Ready, PaneState::Showing(file)),
            (Shown::Visible, Load::Reading, PaneState::Pending),
            (Shown::Visible, Load::Failed, PaneState::Failed),
            (Shown::Hidden, Load::Reading, PaneState::Hidden),
            (Shown::Hidden, Load::Ready, PaneState::Hidden),
        ];
        for (shown, load, want) in cases {
            assert_eq!(pane_state(shown, file, load), want, "{shown:?} {load:?}");
        }
    }
}
