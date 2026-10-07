//! The shell's component sheets, each placed in the `components` section right after the sheet it
//! followed when the components listed them (`ds::style::kit::Sheet`), so the cascade is the one
//! the stylesheet always had.

use ds::style::kit::Sheet;

/// The shell's sheets, in cascade order.
pub(crate) const SHEETS: [Sheet; 30] = [
    Sheet {
        name: "date_picker",
        css: include_str!("date_picker/style.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "app_switcher",
        css: include_str!("switcher/app_switcher.css"),
        after: "alert",
    },
    Sheet {
        name: "banner_stack",
        css: include_str!("notifications/banner_stack.css"),
        after: "avatar",
    },
    Sheet {
        name: "battery_ring",
        css: include_str!("battery/ring.css"),
        after: "avatar",
    },
    Sheet {
        name: "clock_face",
        css: include_str!("clock/face.css"),
        after: "chip",
    },
    Sheet {
        name: "device_glyph",
        css: include_str!("battery/device_glyph.css"),
        after: "count",
    },
    Sheet {
        name: "dock_parts",
        css: include_str!("dock/parts.css"),
        after: "count",
    },
    Sheet {
        name: "dock_tile",
        css: include_str!("dock/tile.css"),
        after: "count",
    },
    Sheet {
        name: "emoji",
        css: include_str!("emoji.css"),
        after: "edit_surface",
    },
    Sheet {
        name: "group_header",
        css: include_str!("notifications/group_header.css"),
        after: "emoji_grid",
    },
    Sheet {
        name: "idle_dim",
        css: include_str!("idle_dim/style.css"),
        after: "icon_view",
    },
    Sheet {
        name: "lock_screen",
        css: include_str!("lock/screen.css"),
        after: "list_row",
    },
    Sheet {
        name: "lock_clock",
        css: include_str!("lock/clock.css"),
        after: "list_row",
    },
    Sheet {
        name: "password_field",
        css: include_str!("lock/password_field.css"),
        after: "list_row",
    },
    Sheet {
        name: "lock_prompt",
        css: include_str!("lock/prompt.css"),
        after: "list_row",
    },
    Sheet {
        name: "menu_bar_item",
        css: include_str!("bar/menu_bar_item.css"),
        after: "menu",
    },
    Sheet {
        name: "notification_card",
        css: include_str!("notifications/card.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "osd",
        css: include_str!("osd.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "module_tile",
        css: include_str!("control_center/module_tile.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "module_panel",
        css: include_str!("control_center/module_panel.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "pane",
        css: include_str!("control_center/pane.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "month_grid",
        css: include_str!("month_grid.css"),
        after: "menu_entry",
    },
    Sheet {
        name: "user_picture",
        css: include_str!("user_picture.css"),
        after: "pdf_thumb",
    },
    Sheet {
        name: "polkit_prompt",
        css: include_str!("lock/polkit_prompt.css"),
        after: "popover",
    },
    Sheet {
        name: "now_playing",
        css: include_str!("now_playing.css"),
        after: "settings_row",
    },
    Sheet {
        name: "shot_thumbnail",
        css: include_str!("thumbs/shot_thumbnail.css"),
        after: "sheet",
    },
    Sheet {
        name: "widget_frame",
        css: include_str!("widget/frame.css"),
        after: "voice_orb",
    },
    Sheet {
        name: "workspace_pills",
        css: include_str!("bar/workspace_pills.css"),
        after: "window_frame",
    },
    Sheet {
        name: "widget_views",
        css: include_str!("widget/views.css"),
        after: "detail_morph",
    },
    Sheet {
        name: "widget_gallery",
        css: include_str!("widget/gallery.css"),
        after: "detail_morph",
    },
];
