//! The crash guard's two jobs on the surface that carries it: mark the start good once the
//! surface has been alive long enough, and say so when the person's styles were left off because
//! the last two starts never got there. The notice is quire's `StylesOffNotice`; its Turn On
//! Again button asks the daemon on the feed's `turn_on` (nothing here opens Settings or turns a
//! style on by itself). A surface that has no [`SettingsFeed`], or whose feed has no guard,
//! shows nothing.

use dioxus::prelude::*;
use ds::prelude::StylesOffNotice;
use ds_blitz::styles_guard::Load;
use ds_blitz::{StylesGuard, use_styles_guard};
use shell_host::use_watch;
use sill_settings::ShellFile;

use crate::settings::{SettingsFeed, TurnOnStyles};

/// What the guard has to say on this surface; put it where a line of text fits.
#[component]
pub fn StylesNotice() -> Element {
    let feed = use_hook(|| {
        try_consume_context::<SettingsFeed>()
            .unwrap_or_else(|| SettingsFeed::fixed(ShellFile::default()))
    });
    let guard = use_watch(feed.guard.clone());
    let turn_on = feed.turn_on.clone();
    let Some(guard) = guard() else {
        return rsx! {};
    };
    let off = guard.load() == Load::Without;
    rsx! {
        StylesAlive { guard }
        if off {
            StylesOffNotice {
                program: "sill",
                onturnon: move |()| turn_on.send(TurnOnStyles),
            }
        }
    }
}

/// Holds quire's wait that marks the start good: a component of its own so the hook is called
/// only when there is a guard.
#[component]
fn StylesAlive(guard: StylesGuard) -> Element {
    use_styles_guard(guard);
    rsx! {}
}
