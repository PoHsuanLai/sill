//! A scripted stand-in for the keyboard, for the debug daemon's acceptance run only: shell-host
//! injects pointer input but no keys, and on the nested cosmic-comp the power
//! menu gets no keyboard focus while the nested window has none on the host, so `wtype` keys
//! never reach it. A debug build started with both
//! `SILL_POWER_MENU_DRY_RUN` and `SILL_POWER_MENU_KEYS` provides this root context
//! (`crates/sill/src/power_menu.rs`); the keys then go through the same [`super::sheet`]
//! machine a real key does. A release build never provides it.

use dioxus::prelude::*;
use tokio::sync::watch;

use crate::surfaces::power_menu::drive::MenuCtx;
use crate::surfaces::power_menu::keys::PowerMenuKey;
use crate::surfaces::power_menu::sheet::SheetInput;

/// The latest scripted key and its count, so the same key twice is two presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScriptedKey {
    pub count: u64,
    pub key: Option<PowerMenuKey>,
}

/// The root context; its sender is the daemon's.
#[derive(Debug, Clone)]
pub struct PowerMenuScript(pub watch::Receiver<ScriptedKey>);

/// A script word as the key it names: `left`, `right`, `up`, `down`, `tab`, `backtab`,
/// `enter`, `escape` (surrounding blanks ignored, any case).
pub fn script_key(word: &str) -> Option<PowerMenuKey> {
    match word.trim().to_ascii_lowercase().as_str() {
        "left" => Some(PowerMenuKey::Left),
        "right" => Some(PowerMenuKey::Right),
        "up" => Some(PowerMenuKey::Up),
        "down" => Some(PowerMenuKey::Down),
        "tab" => Some(PowerMenuKey::Tab),
        "backtab" => Some(PowerMenuKey::BackTab),
        "enter" => Some(PowerMenuKey::Enter),
        "escape" => Some(PowerMenuKey::Escape),
        _ => None,
    }
}

/// Feed every scripted key to the menu, when a script is provided.
pub(crate) fn use_script(ctx: MenuCtx) {
    let script = try_use_context::<PowerMenuScript>();
    use_hook(move || {
        let Some(PowerMenuScript(mut rx)) = script else {
            return;
        };
        spawn(async move {
            while rx.changed().await.is_ok() {
                let scripted = *rx.borrow_and_update();
                if let Some(key) = scripted.key {
                    eprintln!("power_menu.scripted key={key:?}");
                    ctx.input(SheetInput::Key(key));
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::script_key;
    use crate::surfaces::power_menu::keys::PowerMenuKey;

    #[test]
    fn script_words_name_keys() {
        let cases = [
            ("left", Some(PowerMenuKey::Left)),
            ("Right\n", Some(PowerMenuKey::Right)),
            (" up ", Some(PowerMenuKey::Up)),
            ("down", Some(PowerMenuKey::Down)),
            ("tab", Some(PowerMenuKey::Tab)),
            ("backtab", Some(PowerMenuKey::BackTab)),
            ("ENTER", Some(PowerMenuKey::Enter)),
            ("escape", Some(PowerMenuKey::Escape)),
            ("space", None),
            ("", None),
        ];
        for (word, want) in cases {
            assert_eq!(script_key(word), want, "{word:?}");
        }
    }
}
