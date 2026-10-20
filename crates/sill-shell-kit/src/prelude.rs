//! What a shell consumer's `use sill_shell_kit::prelude::*` brings in: the shell components and the
//! widget contract's names, one `pub use` per name. Everything else is reached by its home path.

// Shell components
pub use crate::bar::menu_bar_item::MenuBarItem;
pub use crate::bar::workspace_pills::WorkspacePill;
pub use crate::bar::workspace_pills::WorkspacePills;
pub use crate::battery::device_glyph::DeviceGlyph;
pub use crate::battery::ring::BatteryRing;
pub use crate::control_center::module_grid::ModuleGrid;
pub use crate::control_center::module_panel::ModulePanel;
pub use crate::control_center::module_tile::ModuleTile;
pub use crate::control_center::pane::PaneFooter;
pub use crate::control_center::pane::PaneHeader;
pub use crate::date_picker::view::DatePicker;
pub use crate::dock::parts::DockFloor;
pub use crate::dock::parts::DockLabel;
pub use crate::dock::parts::RunningDot;
pub use crate::dock::tile::DockTile;
pub use crate::emoji::AnimatedEmoji;
pub use crate::lock::clock::LockClock;
pub use crate::lock::polkit_prompt::PolkitPrompt;
pub use crate::lock::prompt::LockPrompt;
pub use crate::lock::screen::LockScreen;
pub use crate::month_grid::MonthGrid;
pub use crate::notifications::banner_stack::BannerStack;
pub use crate::notifications::card::NotificationCard;
pub use crate::notifications::group_header::GroupHeader;
pub use crate::now_playing::NowPlayingTrack;
pub use crate::now_playing::track_position::TrackPosition;
pub use crate::osd::Osd;
pub use crate::switcher::app_switcher::AppSwitcher;
pub use crate::thumbs::shot_ghost::ShotGhost;
pub use crate::thumbs::shot_thumbnail::ShotThumbnail;
pub use crate::user_picture::picker::UserPicturePicker;
pub use crate::widget::card::WidgetCard;
pub use crate::widget::frame::WidgetFrame;

// The widget contract
pub use crate::widget::contract::Widget;
pub use crate::widget::contract::WidgetContext;
pub use crate::widget::contract::WidgetKind;
pub use crate::widget::kind::WidgetSize;
