//! A scripted stand-in for the keyboard, for `examples/launcher_demo.rs` only: shell-host can
//! inject pointer input but no keys (FINDINGS G20), so the demo types into the field and
//! presses Ctrl+K through this root context instead. The daemon never provides it. Arrow keys
//! are not offered: the quire palette moves its own selection and nothing outside it can.

use dioxus::prelude::*;
use tokio::sync::watch;

use crate::surfaces::launcher::events::PanelCtx;
use crate::surfaces::launcher::keys::LauncherKey;

/// One scripted input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptedInput {
    /// The field's text becomes this, as typing would make it.
    Type(String),
    /// Tab, Shift+Tab, Ctrl+K or Escape.
    Key(LauncherKey),
}

/// The latest scripted input and its count, so the same input twice is two inputs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scripted {
    pub count: u64,
    pub input: Option<ScriptedInput>,
}

/// The root context a demo provides; its sender is the demo's.
#[derive(Debug, Clone)]
pub struct LauncherScript(pub watch::Receiver<Scripted>);

/// Feed every scripted input to the panel, when a script is provided.
pub(crate) fn use_script(panel: PanelCtx) {
    let script = try_use_context::<LauncherScript>();
    use_hook(move || {
        let Some(LauncherScript(mut rx)) = script else {
            return;
        };
        spawn(async move {
            while rx.changed().await.is_ok() {
                let input = rx.borrow_and_update().input.clone();
                match input {
                    Some(ScriptedInput::Type(text)) => panel.typed(text),
                    Some(ScriptedInput::Key(key)) => panel.press(key),
                    None => {}
                }
            }
        });
    });
}
