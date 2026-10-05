//! The tray's pure decisions: which art a status item draws (design/08-ICONS.md §1.5), what a
//! press on it asks of the item (SNI `Activate`, `SecondaryActivate`, the dbusmenu), where on
//! the screen that press was, and how far the wheel turned over it.

use dioxus::html::geometry::WheelDelta;
use ds::base::press::{PointerButton, Press};
use ds::components::menus::item::item::MenuImage;
use ds::prelude::*;
use ds::style::icon::classify::ChromaLimit;
use ds::style::icon::classify::IconKind;
use ds::style::icon::classify::classify_with;
use ds::style::icon::url::IconUrl;
use sill_model::tray::{ScrollAxis, ScrollDelta, TrayCommand, TrayIcon, TrayItemId, TrayMenuIcon};
use sill_services::tray::{IconArt, icon_art};

/// The top-left corner of the bar's surface in the compositor's logical space: its output's
/// origin (the bar is anchored to the output's top-left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ScreenOrigin {
    pub x: i32,
    pub y: i32,
}

/// Whether an item has a dbusmenu to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemMenu {
    Present,
    Absent,
}

/// What a press on a status item does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrayAction {
    /// Send this to the item.
    Send(TrayCommand),
    /// Open the item's dbusmenu: the bar's menu session does it on the press itself (and tells
    /// the item its menu is about to show), so the click has nothing left to do.
    OpenMenu,
}

/// `icons.symbolic_chroma_max` (thousandths of OKLCH chroma, quire's appearance file) as the
/// chroma test's limit; quire's default when the value is out of the key's range.
pub(crate) fn chroma_limit(thousandths: u16) -> ChromaLimit {
    ChromaLimit::try_from(f32::from(thousandths) / 1000.0).unwrap_or_default()
}

/// The art `art` as quire draws it: a PNG by the chroma test (`ds::style::icon::classify::classify_with` at
/// `limit`), a theme file by its name, and the window glyph when there is none or its URL is
/// refused. A PNG the test cannot decode is shown as it is.
pub(crate) fn icon_source(art: Option<IconArt>, limit: ChromaLimit) -> IconSource {
    art.and_then(|art| art_source(art, limit, IconSize::Bar))
        .unwrap_or(IconSource::Glyph(Icon::Window))
}

/// The image at the start of a dbusmenu row: its icon as [`icon_source`] draws a status item's,
/// at the menu's size; none when the row has none or the theme has no such name.
pub(crate) fn row_image(icon: &TrayMenuIcon, limit: ChromaLimit) -> Option<MenuImage> {
    let art = match icon {
        TrayMenuIcon::None => None,
        TrayMenuIcon::Named(name) => icon_art(&TrayIcon::Named(name.clone()), ROW_ICON_PX),
        TrayMenuIcon::Png(png) => Some(IconArt::Png(png.clone())),
    }?;
    art_source(art, limit, IconSize::Base).map(MenuImage::Source)
}

/// The size a row's theme icon is looked up at: `IconSize::Base`.
const ROW_ICON_PX: u16 = 16;

/// `art` drawn at `size`, `None` when its file's URL is refused.
fn art_source(art: IconArt, limit: ChromaLimit, size: IconSize) -> Option<IconSource> {
    let external = |url| ExternalIcon { url, size };
    match art {
        IconArt::Png(png) => {
            let url = external(IconUrl::png(&png));
            Some(match classify_with(&png, limit) {
                Ok(IconKind::Symbolic) => IconSource::Symbolic(url),
                Ok(IconKind::Image) | Err(_) => IconSource::Image(url),
            })
        }
        IconArt::SymbolicFile(path) => IconUrl::file(&path)
            .map(|url| IconSource::Symbolic(external(url)))
            .ok(),
        IconArt::ImageFile(path) => IconUrl::file(&path)
            .map(|url| IconSource::Image(external(url)))
            .ok(),
    }
}

/// `press` on `item`: the primary button opens the menu when there is one and activates the
/// item otherwise; the secondary button opens the menu, or asks the item for its own
/// (SNI `ContextMenu`) when it has none; the middle button is SNI's secondary activation. Each
/// carries the press's screen point (SNI's "where to show eventual windows").
pub(crate) fn press_action(
    press: Press,
    item: &TrayItemId,
    menu: ItemMenu,
    origin: ScreenOrigin,
) -> TrayAction {
    let (x, y) = screen_point(origin, press.at);
    let item = item.clone();
    match (press.button, menu) {
        (PointerButton::Primary | PointerButton::Secondary, ItemMenu::Present) => {
            TrayAction::OpenMenu
        }
        (PointerButton::Primary, ItemMenu::Absent) => {
            TrayAction::Send(TrayCommand::Activate { item, x, y })
        }
        (PointerButton::Middle, _) => {
            TrayAction::Send(TrayCommand::SecondaryActivate { item, x, y })
        }
        (PointerButton::Secondary, ItemMenu::Absent) => {
            TrayAction::Send(TrayCommand::ContextMenu { item, x, y })
        }
    }
}

/// A surface-local point in the compositor's logical space, whole pixels.
pub(crate) fn screen_point(origin: ScreenOrigin, at: Point) -> (i32, i32) {
    (
        origin.x + at.x.0.round() as i32,
        origin.y + at.y.0.round() as i32,
    )
}

/// The notches a wheel event turned, on its main axis, in `wl_pointer`'s sign (positive is down
/// or right). shell-host hands Blitz the axis value negated (its `input/pointer.rs`), so a line
/// delta is turned back. A high-resolution wheel's half notch rounds to one; a pixel delta (a
/// touchpad's continuous scroll) and a page delta are not notches and send nothing.
pub(crate) fn scroll_delta(delta: WheelDelta) -> Option<ScrollDelta> {
    let WheelDelta::Lines(lines) = delta else {
        return None;
    };
    let (axis, value) = if lines.y.abs() >= lines.x.abs() {
        (ScrollAxis::Vertical, lines.y)
    } else {
        (ScrollAxis::Horizontal, lines.x)
    };
    let steps = (-value).round() as i32;
    (steps != 0).then_some(ScrollDelta { axis, steps })
}

#[cfg(test)]
mod tests {
    use super::{
        ItemMenu, ScreenOrigin, TrayAction, chroma_limit, icon_source, press_action, row_image,
        screen_point, scroll_delta,
    };
    use dioxus::html::geometry::WheelDelta;
    use dioxus::prelude::Modifiers;
    use ds::base::press::{PointerButton, Press};
    use ds::components::menus::item::item::MenuImage;
    use ds::prelude::*;
    use sill_model::tray::{ScrollAxis, ScrollDelta, TrayCommand, TrayItemId, TrayMenuIcon};
    use sill_services::tray::IconArt;
    use std::path::PathBuf;

    fn png(pixel: [u8; 4]) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba(pixel));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut bytes, image::ImageFormat::Png)
            .expect("encodes");
        bytes.into_inner()
    }

    fn kind(source: &IconSource) -> &'static str {
        match source {
            IconSource::Glyph(Icon::Window) => "window glyph",
            IconSource::Glyph(_) => "other glyph",
            IconSource::Symbolic(_) => "symbolic",
            IconSource::Image(_) => "image",
            IconSource::Status(_) => "status",
        }
    }

    #[test]
    fn each_kind_of_art_is_drawn_its_own_way() {
        let cases = [
            (None, "window glyph"),
            (Some(IconArt::Png(png([40, 40, 40, 255]))), "symbolic"),
            (Some(IconArt::Png(png([220, 30, 30, 255]))), "image"),
            (Some(IconArt::Png(b"not a png".to_vec())), "image"),
            (
                Some(IconArt::SymbolicFile(PathBuf::from("/i/a-symbolic.svg"))),
                "symbolic",
            ),
            (Some(IconArt::ImageFile(PathBuf::from("/i/a.svg"))), "image"),
            (
                Some(IconArt::ImageFile(PathBuf::from("relative.svg"))),
                "window glyph",
            ),
        ];
        for (art, want) in cases {
            let name = format!("{art:?}").chars().take(40).collect::<String>();
            assert_eq!(
                kind(&icon_source(
                    art,
                    ds::style::icon::classify::ChromaLimit::default()
                )),
                want,
                "{name}"
            );
        }
        // A faintly tinted grey (OKLCH chroma about .03) is symbolic at quire's .04 and an image
        // once the key is lowered to .01.
        let tinted = || Some(IconArt::Png(png([120, 110, 100, 255])));
        assert_eq!(kind(&icon_source(tinted(), chroma_limit(40))), "symbolic");
        assert_eq!(kind(&icon_source(tinted(), chroma_limit(10))), "image");
    }

    #[test]
    fn a_rows_icon_is_drawn_at_the_menus_size_by_the_chroma_test() {
        use ds::style::icon::render::IconSize;
        let limit = ds::style::icon::classify::ChromaLimit::default();
        let size = |image: Option<MenuImage>| match image {
            Some(MenuImage::Source(IconSource::Symbolic(icon) | IconSource::Image(icon))) => {
                Some(icon.size)
            }
            _ => None,
        };
        let symbolic = row_image(&TrayMenuIcon::Png(png([40, 40, 40, 255])), limit);
        assert!(matches!(
            symbolic,
            Some(MenuImage::Source(IconSource::Symbolic(_)))
        ));
        assert_eq!(size(symbolic), Some(IconSize::Base));
        let image = row_image(&TrayMenuIcon::Png(png([220, 30, 30, 255])), limit);
        assert!(matches!(
            image,
            Some(MenuImage::Source(IconSource::Image(_)))
        ));
        assert_eq!(size(image), Some(IconSize::Base));
        assert_eq!(row_image(&TrayMenuIcon::None, limit), None);
    }

    fn press(button: PointerButton, x: f32, y: f32) -> Press {
        Press {
            button,
            modifiers: Modifiers::empty(),
            at: Point { x: Px(x), y: Px(y) },
        }
    }

    #[test]
    fn each_button_asks_the_item_its_own_thing_at_the_screen_point() {
        let item = TrayItemId(":1.7/StatusNotifierItem".to_owned());
        let origin = ScreenOrigin { x: 1920, y: 0 };
        let at = |x, y| (item.clone(), x, y);
        let activate = |(item, x, y)| TrayAction::Send(TrayCommand::Activate { item, x, y });
        let secondary =
            |(item, x, y)| TrayAction::Send(TrayCommand::SecondaryActivate { item, x, y });
        let context = |(item, x, y)| TrayAction::Send(TrayCommand::ContextMenu { item, x, y });
        let cases = [
            (
                PointerButton::Primary,
                ItemMenu::Present,
                TrayAction::OpenMenu,
            ),
            (
                PointerButton::Secondary,
                ItemMenu::Present,
                TrayAction::OpenMenu,
            ),
            (
                PointerButton::Primary,
                ItemMenu::Absent,
                activate(at(2031, 16)),
            ),
            (
                PointerButton::Middle,
                ItemMenu::Present,
                secondary(at(2031, 16)),
            ),
            (
                PointerButton::Middle,
                ItemMenu::Absent,
                secondary(at(2031, 16)),
            ),
            (
                PointerButton::Secondary,
                ItemMenu::Absent,
                context(at(2031, 16)),
            ),
        ];
        for (button, menu, want) in cases {
            assert_eq!(
                press_action(press(button, 110.6, 15.5), &item, menu, origin),
                want,
                "{button:?} {menu:?}"
            );
        }
        assert_eq!(
            screen_point(
                ScreenOrigin { x: -1280, y: 200 },
                press(PointerButton::Primary, 3.4, 7.6).at
            ),
            (-1277, 208)
        );
    }

    #[test]
    fn the_settings_key_is_read_in_thousandths_and_out_of_range_is_the_default() {
        const CASES: &[(u16, u16)] = &[(40, 40), (10, 10), (200, 200), (201, 40), (1000, 40)];
        for &(key, want) in CASES {
            assert_eq!(
                chroma_limit(key),
                ds::style::icon::classify::ChromaLimit(want),
                "{key}"
            );
        }
    }

    #[test]
    fn a_notch_is_one_step_in_the_wl_pointer_sign() {
        let v = |steps| {
            Some(ScrollDelta {
                axis: ScrollAxis::Vertical,
                steps,
            })
        };
        let h = |steps| {
            Some(ScrollDelta {
                axis: ScrollAxis::Horizontal,
                steps,
            })
        };
        let cases = [
            // shell-host negates wl_pointer: a notch down arrives as -1 line.
            (WheelDelta::lines(0.0, -1.0, 0.0), v(1)),
            (WheelDelta::lines(0.0, 2.0, 0.0), v(-2)),
            (WheelDelta::lines(0.0, -0.5, 0.0), v(1)),
            (WheelDelta::lines(-1.0, 0.0, 0.0), h(1)),
            (WheelDelta::lines(0.0, -0.2, 0.0), None),
            (WheelDelta::lines(0.0, 0.0, 0.0), None),
            (WheelDelta::pixels(0.0, -12.0, 0.0), None),
            (WheelDelta::pages(0.0, -1.0, 0.0), None),
        ];
        for (delta, want) in cases {
            assert_eq!(scroll_delta(delta), want, "{delta:?}");
        }
    }
}
