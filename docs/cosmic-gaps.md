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
| Colour management: tag surfaces sRGB / Display P3, compositor maps to the panel (P3 monitors look oversaturated otherwise) | No `wp_color_manager_v1`, no `frog_color_management_factory_v1`, no `wp_color_representation_manager_v1`; writes no GAMMA_LUT / DEGAMMA_LUT / CTM | verified: `wayland-info` on nested 1.8.0 and source (sill FINDINGS "Displays") | fork, PR |
| Night light in software (for panels without DDC) | No `zwlr_gamma_control_manager_v1`, no CTM, so gammastep / wlsunset cannot work | verified: same | fork, PR (sill already does it in hardware over DDC) |
| Default scale that prefers whole scales like macOS (2x on 27" 4K) and respects density | cosmic-comp picks 2 for every 4K output regardless of size | verified: source read (sill FINDINGS "Displays") | shell (sill's scale policy writes it via cosmic-randr on first sight) |

## Windows

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| macOS Sequoia tiling: drag to a corner = quarter, green-button menu Fill / Left / Right / Centre, keyboard shortcuts for them | Edge snapping to halves and COSMIC's own tiling mode; no quarter corners or Fill/Centre as described | assumed (from COSMIC behaviour as known, not tested here) | fork, or shell via toplevel management if the protocol can set geometry (it cannot today: assumed) |
| Client-driven placement (Left half / Right half / Centre) for our own windows' green-button menu | No protocol: xdg_toplevel has no placement request, cosmic-comp has no `xdg_toplevel_tag_manager_v1`, and `zcosmic_toplevel_manager_v1` v4 has no tile or placement request (`set_rectangle` is only the minimize target); the menu ships Fill (maximize) only | verified: shell-host F50 (protocol listings and source) | fork (a placement request in the cosmic toplevel manager), PR |
| Client-side resize margins on floating windows | cosmic-comp marks every floating window as tiled on all four edges in every configure, so a client's own resize margin never shows; edge resize is cosmic's | verified: shell-host F48 (configure states observed) | fork or PR (send tiled states only when actually tiled) |
| Option-drag resize symmetric around the centre | Not known to exist | assumed | fork |
| Minimize animation into the dock tile (genie / scale) toward the dock's rect | Needs `set_rectangle` on the foreign toplevel for the dock icon; sill cannot bind it yet (shell-host globals, sill G2), and the animation itself is compositor-side | verified (sill FINDINGS G2 / "minimize target"); animation assumed | shell-host + fork |
| Window zoom / open-close animations like macOS | COSMIC's own animations | assumed | fork |
| Toplevel urgency (a window asking for attention, for dock bounce) | cctk's toplevel info has no urgency | verified: cctk API read (plan, COSMIC research) | fork, PR; LauncherEntry `urgent` as the workaround |
| Stable workspace ids | ext-workspace sends no `id` (`stable_id` empty) | verified: nested run (sill FINDINGS M1) | PR |

## Input and focus

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| Keyboard focus for an `Exclusive` overlay layer (power menu, launcher) when the nested compositor window is unfocused on the host | No `wl_keyboard.enter` sent | verified on nested only (sill G30); real session unverified | check on a real login first |
| Keyboard for a popup opened without an input serial (`sill control-center toggle`, a debug click) | The popup is mapped and kept, but gets no keyboard, so Escape cannot reach it | verified on nested (sill FINDINGS "M5 control center") | check on a real login; PR if it holds there |
| Modifier state for an unfocused layer surface (hot corners that need a held key, as macOS offers) | Protocol-wise only the focused surface gets `wl_keyboard.modifiers`; KWin sends it anyway after `enter`, smithay-based cosmic-comp is expected not to (unverified); asking for keyboard focus would steal it from the window | shell-host F61 (KWin probe); cosmic-comp assumed | fork (send modifiers to the surface under the pointer, or a shell-only protocol), PR |
| Virtual keyboard for automated tests | `wtype` keys never reached clients in the nested session | verified on nested (sill G30) | PR (test harness only) |
| Layer surface under a still pointer gets `enter` | No enter until the pointer moves (cosmic-comp #2230) | upstream issue | PR |

## Rendering and performance

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| Timely frame callbacks for fullscreen layer surfaces | Delayed (cosmic-comp #2777); shell-host has a watchdog | upstream issue | PR |
| Blur on rotated outputs, and cheaper blur | #2531, #2613/#2795 | upstream issues | PR / fork |
| Nested winit backend on NVIDIA that does not run away when its EGL surface is lost | One instance grew to 64 GB in a retry loop (shell-host F39); Mesa default and an RSS watchdog in `nested.sh` | verified | PR (dev only) |

## Session and system

| Want | COSMIC today | Evidence | Route |
| --- | --- | --- | --- |
| A session where sill replaces cosmic-panel, cosmic-bg, cosmic-osd, cosmic-notifications | cosmic-session hard-codes its component list | verified: source (plan, COSMIC research; cosmic-session `epoch-1.8.0` `main.rs` read for M7) | own session script `dist/sill-session` (documented, M7), or fork cosmic-session |
| A lock screen sill can start per lock and hear "locked" from | cosmic-greeter is the locker too, but resident: with logind reachable it waits for logind's `Lock` on its parent's session and never exits; one-shot (lock at once, exit after unlock) only when the system bus is unreachable; "locked" only in the journal (tracing-journald), and a refused lock (`finished`) is not handled (it would sit with no surfaces) | verified: cosmic-greeter `epoch-1.8.0` `main.rs`, `locker.rs`, `logind.rs` read, and run on the nested 1.8.0 (sill FINDINGS "M7") | shell (sill runs it with no system bus and reads its journal line); PR (a `--lock-now` flag, a ready fd, handle `finished`) |
| A locker that dies while locked does not leave the session locked forever | cosmic-comp keeps the lock until another client locks and unlocks (it accepts a new lock once the old client is gone); correct for security, but only a new locker can get the user back in | verified: `session_lock.rs` read, and observed on the nested 1.8.0 | none wanted: sill never kills its locker; `examples/session_locker` clears it on the nested compositor |
| Idle lock at its own time, apart from screen off | cosmic-idle locks 0.5 s after the screen-off fade, through the `LockScreen` system action (default `loginctl lock-session`); only screen off and suspend (AC / battery) have times | verified: cosmic-idle `epoch-1.8.0` `main.rs` and `cosmic-idle-config` read | PR (a lock time), or a sill idle timer over `ext-idle-notify-v1` if a separate time is ever wanted |
| Settings deep links (open COSMIC Settings on a page) | `Exec` takes no page argument | verified (sill FINDINGS F146) | PR, or our own Settings app (M12) |
| Our brightness / volume OSD instead of cosmic-osd's | Replacing the `system_actions` for brightness keys also drops cosmic-settings-daemon's handling and cosmic-osd's popup | verified (sill FINDINGS "Displays") | config (dist/ shortcut files) |
| `cosmic-randr` failing fast off COSMIC | It hangs forever (never exits, never errors) on a compositor without `zwlr_output_manager_v1`, e.g. KWin | verified (sill FINDINGS G51, gdb stack) | PR (a connect timeout / error exit); sill bounds it at 2 s meanwhile |
| Decode COSMIC's default wallpaper format (JPEG XL) | sill's `image` build has no jxl | verified (sill F132) | shell (add jxl), not a COSMIC gap |

### What cosmic-session does that `dist/sill-session` loses (M7)

Read from cosmic-session `epoch-1.8.0`. "Kept" means the script does it another way.

| cosmic-session does | In a sill session | Matters? |
| --- | --- | --- |
| Starts cosmic-comp and hands the other components its Wayland socket through a private fd | cosmic-comp runs the script as its kiosk command; children inherit `WAYLAND_DISPLAY` | no |
| Imports the systemd user environment and pushes the compositor's into it | `dbus-update-activation-environment --systemd` | kept |
| Starts `cosmic-session.target` (binds `graphical-session.target`, XDG autostart) | `dist/sill-session.target`, the same shape | kept |
| Holds logind's `handle-power-key` inhibitor so the power key does not power off at once | `systemd-inhibit --what=handle-power-key` in the script; the key's `PowerOff` system action is `sill power-menu toggle` (`dist/cosmic`) | kept; needs the check in docs/manual-checks.md |
| `com.system76.CosmicSession` on the session bus (`Exit`, `Restart`), which cosmic-osd's logout dialog and COSMIC Settings call | Nothing owns it. sill's power menu logs out through logind's `TerminateSession`; COSMIC Settings' own log-out buttons (if any call it) do nothing | low: sill's power menu is the logout dialog; `LogOut` system action is `sill power-menu open` |
| cosmic-osd: logout / restart / shutdown dialogs, the polkit agent, volume and brightness indicators | sill's power menu, sill's OSD, `sill polkit-agent` (another desktop's agent) | kept, borrowed agent until M11 |
| cosmic-greeter resident as the locker (logind `Lock`, sleep delay) | sill's locker service runs it per lock | kept |
| Restarts every component forever | the script restarts each one, giving up after five quick exits | kept |
| Screen reader (orca) when a11y is on | not started | yes for anyone who needs it; add orca to the script by hand |
| XDG autostart without systemd | only through `xdg-desktop-autostart.target` | no on Fedora (systemd) |
| cosmic-workspaces (the overview), cosmic-files-applet | not started | not until sill has a workspace overview; the files applet is a cosmic-panel part |
| `org.gnome.SessionManager` shim for apps that inhibit logout | none (neither does cosmic-session in 1.8.0: cosmic-osd only calls it) | no |
