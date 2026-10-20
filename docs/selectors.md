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
| `desktop-menu` | the desktop's context menu (Change Wallpaper…) |
| `dock-menu` | a dock tile's context menu |
| `launcher` | the launcher panel |
| `launcher-catcher` | the transparent surface behind the launcher that a press closes it from |
| `notification-center` | the notification center |
| `banners` | the notification banners |
| `screenshot-thumb` | the screenshot thumbnail |
| `osd` | the volume and brightness indicator |
| `power-menu` | the power menu |
| `polkit` | the polkit prompt |
| `confirm` | the confirmation sheet (what intentd asks before the terminal, the assistant or an agent acts) |
| `voice-consent` | the voice consent sheet (asked once, before the microphone first opens) |
| `accounts-sheet` | the accounts sheet (accountd's consent question and add-account steps, in a window of their own) |
| `capture` | the sharing sheet (which screen or window an app may share) |
| `eyedropper` | the colour picker's eyedropper (a still picture of the output with a round magnifier that follows the pointer, the pixel under it ringed) |
| `display-trial` | the question after a program changes the displays: "Keep these display settings?" with a countdown, Revert (the default) and Keep Changes |
| `permission` | the permission sheet (an app asks, through the portal, to take a screenshot or control the computer; Allow or Don't Allow) |
| `switcher` | the app switcher |
| `spaces-bar` | Mission Control's Spaces bar (the Spaces of the output across the top, each over a picture of the Space on the output's wallpaper, and a plus that lights under a dragged window; styled by quire's `.ds-spaces-bar-*` classes, `img.ds-spaces-bar-backdrop` is the wallpaper, `.ds-spaces-bar-add[data-drop=target]` the lit plus) |
| `split-picker` | the Split View picker (which window fills the other half) |
| `force-quit` | Force Quit Applications (the list of running apps and the question before one is ended) |
| `wallpaper` | the wallpaper |
| `lock` | the lock screen |
| `idle-dim` | the idle dim |
| `hot-corner` | a hot corner's transparent square |
