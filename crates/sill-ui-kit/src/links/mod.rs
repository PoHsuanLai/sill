//! The shared channels every surface root gets beside `Services` and `SettingsFeed`, one file
//! per link: the daemon (or another surface) writes one end, the surfaces follow the other. A
//! link is a watch channel or a value over one, never a Dioxus signal. [`Links`] holds them all;
//! the daemon builds it once and hands it to every root.

mod banners;
mod control_center;
mod edit_widgets;
mod hot_corners;
mod launcher;
mod lock;
mod notification_center;
mod osd;
mod places;
mod power_menu;
mod reveal;
mod switcher;

use sill_settings::FocusLink;

pub use banners::{BannerLink, BannerOutput};
pub use control_center::{ControlCenterAsk, ControlCenterLink};
pub use edit_widgets::{EditShown, EditWidgetsLink, WidgetsWrite, WidgetsWriter};
pub use hot_corners::{Corner, CornerHit, CornerLink};
pub use launcher::{
    CLIPBOARD_TITLE, LauncherLink, LinkKey, LinkPress, Opening, PanelHit, ShownRows,
    ShownRowsSection,
};
pub use lock::{LockLink, PromptOutput};
pub use notification_center::{CenterLink, CenterShown};
pub use osd::{ChangeSerial, Level, LevelSource, OsdChange, OsdKind, OsdLatest, OsdLink};
pub use places::PlacementFile;
pub use power_menu::{PowerMenuLink, PowerMenuOpening, Preselect, Showing};
pub use reveal::{OpenSerial, Opened, PanelFocus, Reveal};
pub use switcher::{AskSerial, Dir, SwitcherAsk, SwitcherAsked, SwitcherLink};

/// The daemon's links.
#[derive(Debug, Clone)]
pub struct Links {
    pub focus: FocusLink,
    pub launcher: LauncherLink,
    pub osd: OsdLink,
    pub power_menu: PowerMenuLink,
    pub control_center: ControlCenterLink,
    pub notification_center: CenterLink,
    pub banners: BannerLink,
    /// Every corner surface writes its firings here; the daemon carries them out. The
    /// screenshot thumbnail follows `banners` (it shows on the banners' output).
    pub corners: CornerLink,
    /// `sill switcher ...` asks every switcher surface here; the lock surfaces follow
    /// `lock` for which one shows the password prompt.
    pub switcher: SwitcherLink,
    pub lock: LockLink,
    /// Edit Widgets: which output shows it (the notification center's "Edit Widgets", `sill
    /// widgets edit`), and where its changes are written and its places read.
    pub edit_widgets: EditWidgetsLink,
}
