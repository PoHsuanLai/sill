# What we want from COSMIC that it does not do (yet)

A running list for the likely cosmic-comp fork (or upstream PRs). Each entry says what we want,
what COSMIC does today, how we know, and the cheapest route. Versions: cosmic-comp 1.8.0
(Fedora), sources at `epoch-1.8.0`. Add entries as work hits them; keep the evidence line honest
(`verified` = protocol listing, source read, or observed on this machine; `assumed` = not yet
checked).

Routes: **fork** (patch cosmic-comp), **PR** (worth offering upstream), **shell** (sill can do it
without the compositor), **config** (COSMIC setting or shortcut file).

## Display and colour

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| Colour management: tag surfaces sRGB / Display P3, compositor maps to the panel (P3 monitors look oversaturated otherwise) | No `wp_color_manager_v1`, no `frog_color_management_factory_v1`, no `wp_color_representation_manager_v1`; writes no GAMMA_LUT / DEGAMMA_LUT / CTM | verified: `wayland-info` on nested 1.8.0 and source | fork, PR |
| Night light in software (for panels without DDC) | No `zwlr_gamma_control_manager_v1`, no CTM, so gammastep / wlsunset cannot work | verified: same | fork, PR (sill already does it in hardware over DDC) |
| Default scale that prefers whole scales like macOS (2x on 27" 4K) and respects density | cosmic-comp picks 2 for every 4K output regardless of size | verified: source read | shell (sill's scale policy writes it via cosmic-randr on first sight) |

## Windows

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| macOS Sequoia tiling: drag to a corner = quarter, green-button menu Fill / Left / Right / Centre, keyboard shortcuts for them | Edge snapping to halves and COSMIC's own tiling mode; no quarter corners or Fill/Centre as described | assumed (from COSMIC behaviour as known, not tested here) | fork, or shell via toplevel management if the protocol can set geometry (it cannot today: assumed) |
| Client-driven placement (Left half / Right half / Centre) for our own windows' green-button menu | No protocol: xdg_toplevel has no placement request, cosmic-comp has no `xdg_toplevel_tag_manager_v1`, and `zcosmic_toplevel_manager_v1` v4 has no tile or placement request (`set_rectangle` is only the minimize target); the menu ships Fill (maximize) only | verified: protocol listings and source | fork (a placement request in the cosmic toplevel manager), PR |
| Client-side resize margins on floating windows | cosmic-comp marks every floating window as tiled on all four edges in every configure, so a client's own resize margin never shows; edge resize is cosmic's | verified: configure states observed | fork or PR (send tiled states only when actually tiled) |
| Option-drag resize symmetric around the centre | Not known to exist | assumed | fork |
| Minimize animation into the dock tile (genie / scale) toward the dock's rect | Needs `set_rectangle` on the foreign toplevel for the dock icon; sill cannot bind it yet (shell-host globals, sill G2), and the animation itself is compositor-side | verified (sill FINDINGS G2); animation assumed | shell-host + fork |
| Window zoom / open-close animations like macOS | COSMIC's own animations | assumed | fork |
| Toplevel urgency (a window asking for attention, for dock bounce) | cctk's toplevel info has no urgency | verified: cctk API read | fork, PR; LauncherEntry `urgent` as the workaround |
| Stable workspace ids | ext-workspace sends no `id` (`stable_id` empty) | verified: nested run | PR |

## Input and focus

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| Keyboard focus for an `Exclusive` overlay layer (power menu, launcher) when the nested compositor window is unfocused on the host | No `wl_keyboard.enter` sent | verified on nested only; real session unverified | check on a real login first |
| Keyboard for a popup opened without an input serial (`sill control-center toggle`, a debug click) | The popup is mapped and kept, but gets no keyboard, so Escape cannot reach it | verified on nested | check on a real login; PR if it holds there |
| Modifier state for an unfocused layer surface (hot corners that need a held key, as macOS offers) | Protocol-wise only the focused surface gets `wl_keyboard.modifiers`; KWin sends it anyway after `enter`, smithay-based cosmic-comp is expected not to (unverified); asking for keyboard focus would steal it from the window | KWin probe; cosmic-comp assumed | fork (send modifiers to the surface under the pointer, or a shell-only protocol), PR |
| A confirmation only the person can answer: hardware input only, nothing drawn over it, no synthetic or agent input | A layer surface on a stock compositor is forgeable by anything that can inject input (uinput, an input-method, a virtual keyboard); sill's sheet says `SheetFallback` in its receipt | assumed (design: `quire_trusted_surface_v1`, spec SPEC.md 4.2, not in cosmic-comp 1.8) | fork (a trusted-surface role with hardware-only input and no capture), PR unlikely |
| Virtual keyboard for automated tests | `wtype` keys never reached clients in the nested session | verified on nested | PR (test harness only) |
| Synthetic input that counts as idle activity (so an idle test can wake the seat) | cosmic-comp 1.8 calls `notify_activity` only for backend input events; `zwp_virtual_keyboard_v1` keys never produce `ext_idle_notification_v1.resumed` (unlocking the session does) | verified on nested 1.8.0; cosmic-comp source read | PR (test harness only); sill's acceptance uses `sill debug idle-wake` meanwhile |
| Output power off on the nested (winit) backend | `zwlr_output_power_v1` answers `failed` at once: `get_dpms` / `set_dpms` exist only for KMS surfaces | verified on nested 1.8.0 and cosmic-comp's output_power.rs read | none wanted (dev only); the real check is on a sill login |
| Layer surface under a still pointer gets `enter` | No enter until the pointer moves (cosmic-comp #2230) | upstream issue | PR |
| A press on a layer surface while a popup of that layer holds the grab (right-click again with the desktop menu up) | Expected to end the grab (`popup_done`) and deliver the press, as smithay's popup grab does; if the press arrives with the grab still up, a second popup is a protocol error (`xdg_wm_base` `not_the_topmost_popup`) that ends sill's connection, so sill's desktop menu waits for the first to be gone (FINDINGS G368) | assumed for a real press; the error itself verified on the nested cosmic-comp 1.8.0, with synthetic presses (`sill debug click`) that never reach the compositor's grab | check on a real login |
| A lone modifier tap as a shortcut (double-tap ⌘ summons the companion) | cosmic-comp binds a key with modifiers, never a modifier alone, so `sill companion tap` runs from a unique chord (`Ctrl+Alt+Shift+Super F19`, `dist/cosmic`) that the keyboard remapper sends for a lone Cmd tap; the `Tap` machine in the companion service decides which tap is the second | assumed: the remap's effect on Cmd-click and Cmd-drag is the open spike (docs/manual-checks.md "The companion") | Toshy route B now; a detector in the fork later, with the same `Tap` machine as its reference |

## Rendering and performance

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| Timely frame callbacks for fullscreen layer surfaces | Delayed (cosmic-comp #2777); shell-host has a watchdog | upstream issue | PR |
| Background blur that also boosts saturation (about ×1.8), as macOS's vibrancy does behind widgets and panels; a tunable sigma (quire recommends 22 logical px for widget cards) | cosmic-comp blurs only, with the strength from the theme's frosted setting | quire design/23 (widget card reference fit) | fork |
| Blur on rotated outputs, and cheaper blur | #2531, #2613/#2795 | upstream issues | PR / fork |
| Nested winit backend on NVIDIA that does not run away when its EGL surface is lost | One instance grew to 64 GB in a retry loop; Mesa default and an RSS watchdog in `nested.sh` | verified | PR (dev only) |

## Session and system

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| A session where sill replaces cosmic-panel, cosmic-bg, cosmic-osd, cosmic-notifications | cosmic-session hard-codes its component list | verified: cosmic-session `epoch-1.8.0` `main.rs` read | own session script `dist/sill-session`, or fork cosmic-session |
| A locker that dies while locked does not leave the session locked forever | cosmic-comp keeps the lock until another client locks and unlocks (it accepts a new lock once the old client is gone); correct for security, but only a new locker can get the user back in | verified: `session_lock.rs` read, and observed on the nested 1.8.0 | none wanted: a restarted sill locks again from its crash marker; examples/session_locker clears a leftover lock on the nested compositor |
| Idle lock at its own time, apart from screen off, and a lock that says why it happened | cosmic-idle locks 0.5 s after the screen-off fade, through the `LockScreen` system action (default `loginctl lock-session`), so every lock reaches the locker as the same logind `Lock` as Super+Escape; only screen off and suspend (AC / battery) have times | verified: cosmic-idle `epoch-1.8.0` `main.rs` and `cosmic-idle-config` read | shell: sill owns idle, locks with a known cause, and leaves cosmic-idle out of `dist/sill-session` |
| One screen-off time per power source (macOS has display-off on battery and on power adapter) | `com.system76.CosmicIdle` has one `screen_off_time`; only suspend has AC / battery times | verified: `cosmic-idle-config` read | shell (`idle.times = "sill"`); PR if COSMIC Settings should show it |
| An Inhibit portal in a COSMIC session (Flatpak apps' "keep the screen on") | `xdg-desktop-portal-cosmic` 1.8 has no `org.freedesktop.impl.portal.Inhibit`; `cosmic-portals.conf` falls back to `gtk`, whose Inhibit talks to GNOME's session manager and, without it, `org.freedesktop.ScreenSaver` | verified: `/usr/share/xdg-desktop-portal/portals/cosmic.portal` and `cosmic-portals.conf` on this machine; the GTK portal's fallback assumed | shell (sill serves `org.freedesktop.ScreenSaver`), PR (an Inhibit portal in xdg-desktop-portal-cosmic) |
| Dim before screen off as a brightness step, not a black overlay | cosmic-idle fades a black layer surface in over 5 s after the screen-off time, then powers off; no protocol lets a client lower an output's brightness | verified: cosmic-idle fade_black.rs read (`FADE_TIME` 5 s); wayland-info on the nested 1.8.0 (no gamma or brightness protocol) | shell (sill's own dim overlay) |
| Settings deep links (open COSMIC Settings on a page) | The main entry's `Exec` (`cosmic-settings`) takes no page argument, so the launcher's Settings rows open its first page; but each page has a hidden desktop entry of its own (`com.system76.CosmicSettings.Wallpaper.desktop`: `Exec=cosmic-settings wallpaper`, `NoDisplay`, `OnlyShowIn=COSMIC`), which the desktop menu's Change Wallpaper launches | verified: `/usr/share/applications` on this machine and the sill acceptance (a stand-in entry); the real entry not launched | shell (the launcher's rows could launch the page's entry too), PR for a documented page argument |
| Our brightness / volume OSD instead of cosmic-osd's | Replacing the `system_actions` for brightness keys also drops cosmic-settings-daemon's handling and cosmic-osd's popup | verified | config (dist/ shortcut files) |
| `cosmic-randr` failing fast off COSMIC | It hangs forever (never exits, never errors) on a compositor without `zwlr_output_manager_v1`, e.g. KWin | verified (gdb stack) | PR (a connect timeout / error exit); sill bounds it at 2 s meanwhile |
| Decode COSMIC's default wallpaper format (JPEG XL) | sill's `image` build has no jxl | verified | shell (add jxl), not a COSMIC gap |

### What cosmic-session does that `dist/sill-session` loses

Read from cosmic-session `epoch-1.8.0`. "Kept" means the script does it another way.

| cosmic-session does | In a sill session | Matters? |
| --- | --- | --- |
| Starts cosmic-comp and hands the other components its Wayland socket through a private fd | cosmic-comp runs the script as its kiosk command; children inherit `WAYLAND_DISPLAY` | no |
| Imports the systemd user environment and pushes the compositor's into it | `dbus-update-activation-environment --systemd` | kept |
| Starts `cosmic-session.target` (binds `graphical-session.target`, XDG autostart) | `dist/sill-session.target`, the same shape | kept |
| Holds logind's `handle-power-key` inhibitor so the power key does not power off at once | `systemd-inhibit --what=handle-power-key` in the script; the key's `PowerOff` system action is `sill power-menu toggle` (`dist/cosmic`) | kept; needs the check in docs/manual-checks.md |
| `com.system76.CosmicSession` on the session bus (`Exit`, `Restart`), which cosmic-osd's logout dialog and COSMIC Settings call | Nothing owns it. sill's power menu logs out through logind's `TerminateSession`; COSMIC Settings' own log-out buttons (if any call it) do nothing | low: sill's power menu is the logout dialog; `LogOut` system action is `sill power-menu open` |
| cosmic-osd: logout / restart / shutdown dialogs, the polkit agent, volume and brightness indicators | sill's power menu, sill's OSD, sill's own polkit agent | kept |
| cosmic-greeter resident as the locker (logind `Lock`, sleep delay) | sill's own lock screen follows logind's `Lock` and holds the sleep delay inhibitor | kept |
| cosmic-idle: the screen-off fade, output power off, idle lock, idle suspend on AC / battery, `org.freedesktop.ScreenSaver` inhibits | no longer started: sill's own idle service does each, reading COSMIC's idle times so COSMIC Settings' Power page keeps working. The dim before screen off is sill's own overlay (quire `IdleDim`), never a brightness change | kept |
| Restarts every component forever | the script restarts each one, giving up after five quick exits | kept |
| Screen reader (orca) when a11y is on | not started | yes for anyone who needs it; add orca to the script by hand |
| XDG autostart without systemd | only through `xdg-desktop-autostart.target` | no on Fedora (systemd) |
| cosmic-workspaces (the overview), cosmic-files-applet | not started | not until sill has a workspace overview (`sill workspace overview` answers `unsupported`; `next` and `prev` work); the files applet is a cosmic-panel part |
| `org.gnome.SessionManager` shim for apps that inhibit logout | none (neither does cosmic-session in 1.8.0: cosmic-osd only calls it) | no |
