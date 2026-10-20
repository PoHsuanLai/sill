//! The app switcher (design/13 §13.3.5, design/20 §1.11): Cmd+Tab over applications, not windows.
//! The window switcher shortcuts run `sill switcher next|prev`,
//! which the daemon writes to the [`sill_ui_kit::links::SwitcherLink`]; the switcher surface on the chosen output takes
//! the keyboard (`Exclusive`) at once, so it sees the chord's modifier go up (Alt's or Super's
//! key-up, or shell-host's modifiers on focus: `hold.rs`, `watch.rs`), and runs
//! [`Opening`](opening::Opening), quire's switcher machine (the one the compositor runs too): a quick tap
//! switches to the previous app with no panel, a hold past `switcher.show_delay_ms` fades quire's `AppSwitcher` in (the apps from
//! `apps::switcher_apps`), Tab and the arrows move, Q quits and H hides the selected app,
//! Escape cancels, the release activates. The machine's outputs become `WlCommand`s through
//! `apps::{activate, quit, hide}`.
//!
//! One warm, hidden Overlay per output (`surface.rs`), mapped from the chord until
//! it closes, drawing nothing until the panel shows: quire's `AppSwitcher` in `Ds { material:
//! Osd, chrome: Transparent }`, fed by `tiles.rs`.

mod apps;
mod hold;
mod io;
mod keys;
mod model;
mod opening;
mod pointer;
mod session;
mod surface;
mod tiles;
mod view;
mod watch;

pub use surface::Switcher;
pub use view::SWITCHER_CSS;
