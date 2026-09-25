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
| A session where sill replaces cosmic-panel, cosmic-bg, cosmic-osd, cosmic-notifications | cosmic-session hard-codes its component list | verified: source (plan, COSMIC research) | own session script (documented), or fork cosmic-session |
| Settings deep links (open COSMIC Settings on a page) | `Exec` takes no page argument | verified (sill FINDINGS F146) | PR, or our own Settings app (M12) |
| Our brightness / volume OSD instead of cosmic-osd's | Replacing the `system_actions` for brightness keys also drops cosmic-settings-daemon's handling and cosmic-osd's popup | verified (sill FINDINGS "Displays") | config (dist/ shortcut files) |
| Decode COSMIC's default wallpaper format (JPEG XL) | sill's `image` build has no jxl | verified (sill F132) | shell (add jxl), not a COSMIC gap |
