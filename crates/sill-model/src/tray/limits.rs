//! How much of a status item the tray holds. An item is another program's registration and its
//! menu is another program's tree, so each is bounded where it enters; the bar draws what is
//! left without having to defend itself.

/// How many items the tray holds: a bar has room for a dozen icons, and a program that registers
/// hundreds is not asking for them to be seen.
pub const TRAY_ITEMS_MAX: usize = 32;

/// How many rows one item's menu holds, submenus counted: a menu longer than this is not read.
pub const MENU_ROWS_MAX: usize = 200;

/// How deep submenus nest.
pub const MENU_DEPTH_MAX: usize = 4;

/// The characters of a row's label or an item's title that are kept.
pub const MENU_LABEL_MAX_CHARS: usize = 120;

/// The largest `icon-data` of a menu row kept, in bytes.
pub const MENU_ICON_MAX_BYTES: usize = 64 * 1024;

/// The longest side of an item's icon pixmap kept, in pixels: a tray icon is drawn at about 22.
pub const PIXMAP_MAX_SIDE: u32 = 256;

/// The longest theme icon name kept, in bytes: real names are a few dozen, and the name is joined
/// onto a path.
pub const TRAY_ICON_NAME_MAX_BYTES: usize = 128;

/// The largest icon file read from the theme, in bytes: a tray icon is drawn at about 22 pixels,
/// so a PNG past this is not an icon.
pub const TRAY_ICON_FILE_MAX_BYTES: u64 = 1024 * 1024;
