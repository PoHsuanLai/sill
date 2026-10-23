//! The person's styles: `~/.config/quire/style.css` (every quire app) and sill's own
//! `~/.config/sill/style.css`, read at start, watched, and published to the feeds every surface
//! root draws them from (ARCHITECTURE.md section 8). The [`StyleKeeper`] holds both files' stores
//! and the two feeds: the sheets, and the crash guard of this start's sheets
//! (`ds_blitz::StylesGuard`), which turns the person's styles off when the last two starts with
//! the same sheets never got a frame up and five seconds alive. Nothing is automatic: a start the
//! guard turned them off for says so on the bar, and its Turn On Again button asks for
//! [`StyleKeeper::turn_on_again`].
//!
//! The watches send each settled text to the host thread, which publishes it; `sill reload`
//! re-reads both. A missing file is an empty style; text that is not UTF-8 keeps the last good
//! text and is reported on stderr like any damaged settings file. Nothing here parses the CSS:
//! `sill style check` reports what is wrong in it. A `url()` the reach rule cut is logged once
//! per change, one line each, so the person can see why an image did not show.
//!
//! The look is the third sheet of the cascade; it stays empty until `appearance.look` lands,
//! which fills `Styles::look` and `StyleHomes::look` in [`load_styles`] and nowhere else.

use std::path::Path;

use ds::style::kit::{LookStyle, StyleHomes, Styles, UserStyle};
use ds_blitz::{StylesGuard, TokioSpawner};
use ds_settings::schema::KeyPath;
use ds_settings::{AppearanceFile, SettingsError, SettingsWriter, Store, WatchState};
use shell_host::{HostEvent, HostHandle};
use sill_ui_kit::links::Outbox;
use sill_ui_kit::settings::TurnOnStyles;
use tokio::runtime::Runtime;
use tokio::sync::watch;

use super::store::report;
use crate::daemon::msg::{Sheet, ShellMsg};

/// The name the guard keeps its record under, and the notice names.
const PROGRAM: &str = "sill";

/// The three stores a style comes from, the feeds they are published to, and the guard.
#[derive(Debug)]
pub struct StyleKeeper {
    desktop: Store,
    app: Store,
    appearance: Store,
    styles: watch::Sender<Styles>,
    guard: watch::Sender<Option<StylesGuard>>,
}

impl StyleKeeper {
    /// Read both files from `desktop` and `app` and begin the guard in `state_dir` (before any
    /// surface loads the styles). `appearance` is the store Turn On Again writes.
    pub fn start(desktop: Store, app: Store, appearance: Store, state_dir: &Path) -> StyleKeeper {
        let styles = load_styles(&desktop, &app);
        report_dropped(&styles);
        let guard = StylesGuard::begin(state_dir, PROGRAM, &styles);
        StyleKeeper {
            desktop,
            app,
            appearance,
            styles: watch::channel(styles).0,
            guard: watch::channel(Some(guard)).0,
        }
    }

    /// The receivers the surfaces' `SettingsFeed` holds: the sheets and the guard.
    pub fn feeds(
        &self,
    ) -> (
        watch::Receiver<Styles>,
        watch::Receiver<Option<StylesGuard>>,
    ) {
        (self.styles.subscribe(), self.guard.subscribe())
    }

    /// Watch both files on `runtime`, sending each settled text to `host`.
    pub fn spawn_watches(&self, runtime: &Runtime, host: &HostHandle<ShellMsg>) {
        spawn_style_watch(runtime, &self.desktop, Sheet::Desktop, host.clone());
        spawn_style_watch(runtime, &self.app, Sheet::App, host.clone());
    }

    /// Re-read both files and publish them.
    pub fn reload(&self) {
        self.replace(load_styles(&self.desktop, &self.app));
    }

    /// `sheet`'s new text, published.
    pub fn set(&self, sheet: Sheet, text: UserStyle) {
        let mut next = self.styles.borrow().clone();
        match sheet {
            Sheet::Desktop => next.desktop = text,
            Sheet::App => next.app = text,
        }
        self.replace(next);
    }

    /// `event`'s change, published, if it is a watched style (`ShellMsg::Style`) or the person's
    /// Turn On Again (`ShellMsg::TurnOnStyles`); anything else is a no-op.
    pub fn publish(&self, event: &HostEvent<ShellMsg>) {
        match event {
            HostEvent::App(ShellMsg::Style(sheet, text)) => self.set(*sheet, (**text).clone()),
            HostEvent::App(ShellMsg::TurnOnStyles) => self.turn_on_again(),
            _ => {}
        }
    }

    /// Turn On Again: `appearance.custom_styles` back to `use`, the failed starts forgotten, and
    /// this start's sheets guarded afresh.
    pub fn turn_on_again(&self) {
        if let Err(error) = write_use(&self.appearance) {
            eprintln!("sill: turning my styles on again: {error}");
        }
        let styles = self.styles.borrow().clone();
        self.guard
            .send_modify(|guard| *guard = guard.as_ref().map(|old| old.turn_on_again(&styles)));
    }

    /// Publish `next` when it differs: a change is logged for what the reach rule cut from it,
    /// and gives the guard its fresh try (`StylesGuard::changed`).
    fn replace(&self, next: Styles) {
        let changed = self.styles.send_if_modified(|now| {
            if *now == next {
                return false;
            }
            now.clone_from(&next);
            true
        });
        if !changed {
            return;
        }
        report_dropped(&next);
        self.guard
            .send_modify(|guard| *guard = guard.as_ref().map(|old| old.changed(&next)));
    }
}

/// Both files as the stores have them now, and the folder each may read pictures from (the
/// folder of the file). The look is empty until `appearance.look` lands.
pub fn load_styles(desktop: &Store, app: &Store) -> Styles {
    let own = app.load::<UserStyle>();
    report(&own);
    let wide = desktop.load::<UserStyle>();
    report(&wide);
    Styles {
        look: LookStyle::default(),
        desktop: wide.value,
        app: own.value,
        homes: StyleHomes {
            look: None,
            desktop: desktop.dir(),
            app: app.dir(),
        },
    }
}

/// One line on stderr for each reference the reach rule cut from `styles`.
fn report_dropped(styles: &Styles) {
    for cut in styles.dropped() {
        eprintln!(
            "sill: {} style.css line {}: {} is outside its folder and was not loaded",
            cut.slot.attribute(),
            cut.cut.line,
            cut.cut.target
        );
    }
}

/// `appearance.custom_styles = "use"`, in place, keeping the rest of the file.
fn write_use(store: &Store) -> Result<(), SettingsError> {
    SettingsWriter::of::<AppearanceFile>(store)?.patch(
        &KeyPath("appearance.custom_styles".to_owned()),
        &toml::Value::String("use".to_owned()),
    )
}

/// Watch `store`'s `style.css` on `runtime`, sending each settled text to `host` as `sheet`'s. A
/// directory that cannot be watched is logged and the style then changes only through
/// `sill reload`.
fn spawn_style_watch(runtime: &Runtime, store: &Store, sheet: Sheet, host: HostHandle<ShellMsg>) {
    let mut watch = store.watch::<UserStyle>(&TokioSpawner::on(runtime.handle().clone()));
    if let WatchState::Blind { reason } = watch.state() {
        eprintln!("sill: watching the style: {reason}");
        return;
    }
    runtime.spawn(async move {
        while let Some(loaded) = watch.changed().await {
            report(&loaded);
            if host
                .send(ShellMsg::Style(sheet, Box::new(loaded.value)))
                .is_err()
            {
                break;
            }
        }
    });
}

/// Follow the Turn On Again button on `turn_on` on `runtime`, telling `host` for each press.
pub fn spawn_turn_on_follower(
    runtime: &Runtime,
    turn_on: &Outbox<TurnOnStyles>,
    host: HostHandle<ShellMsg>,
) {
    let mut pressed = turn_on.subscribe();
    runtime.spawn(async move {
        while pressed.changed().await.is_ok() {
            let sent = pressed.borrow_and_update().is_some();
            if sent && host.send(ShellMsg::TurnOnStyles).is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod tests;
