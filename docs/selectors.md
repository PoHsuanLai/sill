# Surface names in your style.css

Your stylesheet is `~/.config/sill/style.css`. It is read live: save the file and every surface
restyles, delete it and they return to the design system's own look. `sill style check [FILE]`
reports what is wrong or worth knowing in it.

Every surface's root carries `data-surface`, so `.ds[data-surface=bar]` is the bar's root. The rest of the public
selector surface (component classes, attribute axes, token variables) is quire's `docs/selectors.md`.
Token overrides are the recommended form: `.ds { --accent: ... }` restyles every surface at once.

Renaming a surface below is a breaking change for your file, so it is named in the release notes.

| `data-surface` | What it is |
| --- | --- |
| `bar` | the menu bar |
| `bar-menu` | a menu the bar opens (a menu-bar menu, a tray item's menu) |
| `control-center` | the control center and a module's own popup |
| `dock` | the dock |
| `desktop-menu` | the desktop's context menu (Change Wallpaper, Edit Widgets, Show View Options) |
| `dock-menu` | a dock tile's context menu |
| `launcher` | the launcher panel |
| `launcher-catcher` | the transparent surface behind the launcher that a press closes it from |
| `notification-center` | the notification center |
| `banners` | the notification banners |
| `screenshot-thumb` | the screenshot thumbnail |
| `osd` | the volume and brightness indicator |
| `power-menu` | the power menu |
| `polkit` | the polkit prompt |
| `confirm` | the confirmation sheet (what intentd asks before the terminal, the companion or an agent acts) |
| `voice-consent` | the voice consent sheet (asked once, before the microphone first opens) |
| `accounts-sheet` | the accounts sheet (accountd's consent question and add-account steps, in a window of their own) |
| `capture` | the sharing sheet (which screen or window an app may share) |
| `switcher` | the app switcher |
| `spaces-bar` | Mission Control's Spaces bar (the Spaces of the output across the top, and a plus) |
| `split-picker` | the Split View picker (which window fills the other half) |
| `desktop-widgets` | the desktop widgets |
| `edit-widgets` | the widget editor |
| `wallpaper` | the wallpaper |
| `lock` | the lock screen |
| `idle-dim` | the idle dim |
| `hot-corner` | a hot corner's transparent square |
