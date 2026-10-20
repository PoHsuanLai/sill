//! The shared channels every surface root gets beside `Services` and `SettingsFeed`, one file
//! per link: the daemon (or another surface) writes one end, the surfaces follow the other. A
//! link is a watch channel or a value over one, never a Dioxus signal. [`Links`] holds them all;
//! the daemon builds it once and hands it to every root.

mod actions;
mod banners;
mod control_center;
mod display_trial;
mod eyedropper;
mod focus;
mod force_quit;
mod hot_corners;
mod launcher;
mod lock;
mod notification_center;
mod osd;
mod outbox;
mod power_menu;
mod reveal;
mod shared;
mod spaces_bar;
mod split_picker;
mod switcher;
mod window_menu;

pub use actions::{ActionLink, ActionSent, Asked, Origin};
pub use banners::{BannerLink, BannerOutput, KeyEvent, KeyWhat, Transient};
pub use control_center::{ControlCenterAsk, ControlCenterLink};
pub use display_trial::{DisplayTrialLink, TrialAnswered, TrialReply, TrialRun, TrialSerial};
pub use eyedropper::{EyedropFeed, EyedropperLink};
pub use focus::{Delivery, FocusLink};
pub use force_quit::{
    ForceQuitAsk, ForceQuitAsked, ForceQuitLink, ForceQuitOpening, ForceQuitShown,
};
pub use hot_corners::{Corner, CornerHit, CornerLink};
pub use launcher::{
    CLIPBOARD_TITLE, LauncherAsk, LauncherLink, LauncherOpening, PanelHit, ShownRows,
    ShownRowsSection,
};
pub use lock::{LockLink, PromptOutput};
pub use notification_center::{CenterLink, CenterOpening};
pub use osd::{ChangeSerial, Level, LevelSource, OsdChange, OsdKind, OsdLatest, OsdLink};
pub use outbox::{Outbox, Sent};
pub use power_menu::{PowerMenuLink, PowerMenuOpening, PowerMenuPayload, Preselect, Showing};
pub use reveal::{OpenSerial, Opened, Opening, PanelFocus, PanelLink, Reveal};
pub(crate) use shared::Shared;
pub use spaces_bar::{SpacesBarFeed, SpacesBarLink, SpacesBarPicked};
pub use split_picker::{
    OfferSerial, SplitAnswer, SplitAnswered, SplitOffer, SplitPickerLink, SplitReply,
};
pub use switcher::{AskSerial, Asks, Dir, SwitcherAsk, SwitcherAsked, SwitcherLink};
pub use window_menu::{
    MenuSerial, WindowMenuLink, WindowMenuOpening, WindowMenuPick, WindowMenuPicked,
};

/// The daemon's links.
#[derive(Debug, Clone)]
pub struct Links {
    /// Every surface asks for an action here (`sill::actions` carries it out).
    pub actions: ActionLink,
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
    /// A window's menu (the pointer rested on its green light): the daemon opens it, the window
    /// menu surfaces show it, and a pick goes back to the daemon.
    pub window_menu: WindowMenuLink,
    /// The Force Quit Applications panel: the daemon opens it and lists the apps, the panel
    /// surfaces show it, and a confirmed app goes back to the daemon.
    pub force_quit: ForceQuitLink,
    /// The Split View being chosen (a window went full screen on one half): the daemon opens
    /// the offer, the picker surfaces show it, and the answer goes back to the daemon.
    pub split_picker: SplitPickerLink,
    /// Mission Control's Spaces bar: the daemon follows the overview and the desktop onto it,
    /// the bar surfaces draw it, and a click goes back to the daemon.
    pub spaces_bar: SpacesBarLink,
    /// The colour picker's eyedropper: the daemon holds the still pictures of the screens for
    /// the colour being asked for, the eyedropper surfaces draw them.
    pub eyedropper: EyedropperLink,
    /// The display trial (a program changed the displays and the compositor asks whether to keep
    /// them): the daemon opens it, the surface on the banners' output shows it, and the answer
    /// goes back to the daemon.
    pub display_trial: DisplayTrialLink,
}
