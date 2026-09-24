# Findings

Numbered, never renumbered. Each says what was decided, what it departs from, and why. "Q"
marks a quire gap, "G" a shell-host gap: things sill needs that the dependency does not offer,
reported rather than patched here (ORCHESTRATION coherence rule 3).

## W0 freeze (2026-09-24)

- **F1 Dependencies.** `jiff = "0.2"` is proposed for the pinned block (root `Cargo.toml`,
  "PROPOSED FOR THE PINNED BLOCK"): the bar clock needs the local time zone, which `std` cannot
  answer. jiff 0.2.37 is already in the lock graph through `cosmic-dbus-networkmanager`, so it
  adds no package. `ds-settings` is added as a sibling path dependency beside `ds` (not a pinned
  crate), and `sill-settings` as a workspace member.
- **F2 `sill-settings` is a crate.** design/22 §4 allows a module or a crate. `ds-settings`
  depends on dioxus unconditionally (for `use_environment`), so a settings module would put
  dioxus into `sill-services`. `sill-services` stays UI-free, now enforced by a third
  `check-boundary.sh` rule; surfaces receive settings through a second root context,
  `SettingsFeed`.
- **F3 `switcher` and `sound` are domains of their own.** design/22 §4.3 nests them in
  `MenusSettings`, but the catalogue's paths are `switcher.*` and `sound.*`, §2's file table lists
  `switcher` as a domain, and the schema derive's path is always `<domain>.<field>`: nesting would
  make them `menus.switcher.*`. `ShellFile` gains `[switcher]` and `[sound]` tables.
- **F4 `scroll.scrollbars`** is listed in §3.8's scrollbar table with a `scroll.` path; it is a
  field of `ScrollSettings` so the path matches.
- **F5 One-variant enums cannot be settings.** The derive (and `kind_from_variants`) refuses an
  enum with fewer than two variants. `launcher.material` and `control_center.material`
  (`Material::{Sheet}`) became `PanelMaterial::{Sheet, Popover}`, Sheet default: design/20 §1.3
  and §1.5 name Popover for the same panels. `spaces.lookup_order` gained `ByIndexOnly`, the
  future variant §3.14 itself names.
- **F6 `notifications.banner_material`** is sill's (`[notifications]`), following quire FINDINGS
  F15 and ds-settings' `SILL_OWNED` list, not §3.1's "stored under appearance". Same for
  `launcher.material` and `control_center.material`.
- **F7 `spaces.material_tint_alpha`** is not a field: §3.14 calls it an alias of
  `appearance.material_tint_alpha`, "kept in this domain's table for discoverability". The keys
  test excludes it by name, as ds-settings excludes `motion.level`.
- **F8 Text keys are `String`.** The derive recognises text only by the type name `String`, so
  the doc's `SoundTheme(String)`/`SoundName(String)` newtypes would be taken for enums. Q5.
- **F9 Numbers the docs leave open.** Rank prefix boost 0.1 (the plan names 0.7/0.3 and "prefix
  boost" only); frecency bucket boundaries 4/14/31/90 days (Firefox's; the plan names the
  weights); every numeric settings key needs a `range` for the derive, so ranges were chosen
  where §3 gives none (each is in the key's `#[settings]`); the clock reads `Thu 24 Sep 14:05`,
  24-hour, English, since no clock-format key exists; the drag threshold is
  `dock.hold_to_menu_move_px` (both 8 px, A6).
- **F10 Settings pages.** §5 names pages for Basic keys only. Advanced-only domains got: bar,
  launcher, menus, control center on Appearance; scrollbar with scroll on Mouse & Gestures;
  switcher on Keyboard & Shortcuts; sound with notifications on Notifications; spaces on Spaces.
- **F11 Deep links need unique wording.** `deep_link` resolves a query only when exactly one
  key's label or help mentions it, so no dock help text but `dock.magnification`'s says
  "magnification" (§9.6 requires it to resolve).
- **F12 Banner lifetime.** design/13 §13.3.6 says 5000 ms; design/22
  (`notifications.banner_hold_ms`) and design/20 §1.6 (`ToastHold`) say 5200. The key wins.
- **F13 `magnify` layout.** design/10 §10.5.2 extends the outer slots to the pill padding, which
  makes the rest layout move with the pointer inside those slots. Slots here are all `T + g`, the
  pointer is clamped to the outermost slot edges, and a gap wider than `g` (the Trash separator
  block) keeps its rest width and follows the pointer through it. The anchor invariant and
  continuity are tested by deterministic sweeps (1-30 tiles), not proptest (not in the block).
  Geometry is `Cpx`, hundredths of a pixel, so the doc's 0.1 px table holds with integer types.
  `magnify` lives in `sill-services` (plan, brief); the machine in `sill-surfaces` (design/10).
- **F14 Dock machine.** Differences from design/10 §10.4's sketch: the state is `DockState`
  (the component is `Dock`); pointer events carry the component's hit test (`Hit`, `DropZone`)
  so the machine needs no layout; `Release` carries the tile's `TileApp` so the click's action
  (§10.3.6) is decided in the machine; `Tick` + `WakeAt` are the one-shot timer the doc mentions
  in prose; `Reveal::Fixed` is auto-hide off; hover states hold linear progress and
  `displayed()` eases it; input and blur regions are `Element` specs resolved every frame by
  shell-host, so region effects are emitted only by auto-hide transitions; `Redraw` carries no
  layout (the component calls `displayed` and `magnify`).
- **F15 Menu tracking lives in the bar.** design/13 §13.4/§13.7 put `MenuTrack` in quire
  (`overlay/menu_track.rs`, driving every ds `Menu`); quire has no such module. The machine is
  `sill-surfaces/src/surfaces/bar/menu_track/` until quire adopts it (Q1). Arrow keys within a
  menu, wrap, disabled skipping and filtering stay with the quire `Menu` (06 §21 decision 3);
  the machine handles Escape, Left, Right, Enter.
- **F16 Easing evaluation.** `ds::CubicBezier` is token data with no evaluator; timer-driven
  motion (magnification progress, auto-hide slide) needs one. `sill_services::ease` takes the
  token's control points (Q2).
- **F17 Space looks.** design/21 §10 stores per-workspace looks in `quire/spaces.json` through
  "the ds-settings writer"; quire has no such store or reader. `use_space_look` is a stub that
  M1 fills with `ds::default_look(index)` until it exists (Q3).
- **F18 Settings I/O duplication.** ds-settings' `load`, `save` and `watch` are fixed to
  `appearance.toml`; only `lenient` is generic. `sill-settings` reuses `lenient`,
  `SettingsError` and `DEBOUNCE` and repeats the ~60-line atomic write and watch (Q4).
- **F19 Plan and docs disagree; what was chosen.**
  - Dock IPC `Dock{Attention{app, kind}}` (design/10 §10.3.5) and `sill switcher next|prev`
    (design/13 §13.3.5) are not added: the plan's `Request` shape is frozen. The machine accepts
    `Attention` events for when they are.
  - Dock anchoring: the plan says `BOTTOM`; the layer is `BOTTOM|LEFT|RIGHT`, full width, so the
    surface never resizes with magnification (input region follows the `dock-hit` element).
  - Settings file: design/10 §10.6 says `settings.json`; design/22 §2 (the authority on paths)
    says `settings.toml`.
  - `magnify` signature: the plan's `magnify(centers, pointer_x, Params)` versus design/10's
    `magnify(centers, pointer_x, progress, p)`; the doc's form, which the curve needs.
  - Launcher material: design/22 Sheet, design/20 Popover; a key with Sheet default (F5).
  - No settings key names the wallpaper file; M2 reads COSMIC's background config until one
    exists.

## M1 svc-cosmic (2026-09-24)

- **F20 The `cosmic_wl` thread.** Its own `wayland_client::Connection` (shell-host FINDINGS: a
  protocol object cannot cross connections, and the host's connection is the host's), a calloop
  loop over it, and a second small thread that forwards `Control<WlCommand>` (a tokio channel,
  frozen) into a calloop channel so one loop wakes for both. Every cctk callback is reduced
  through `reduce_toplevels`/`reduce_workspaces` inside `watch::Sender::send_if_modified`, so a
  receiver wakes only when a reduction changed something. Toplevels publish on each handle's
  `done` (that is ext-foreign-toplevel's unit of atomicity), workspaces on the manager's `done`
  and again whenever an output is added or renamed (an output's name can arrive after the groups
  on it). Workspaces in a group are ordered by `coordinates`.
- **F21 The degrade path is empty channels, not an `Unavailable` value.** The frozen `Toplevels`
  and `Workspaces` have no availability word, so on a compositor that offers neither
  `ext_foreign_toplevel_list_v1` nor `ext_workspace_manager_v1` (KWin 6 here offers neither;
  checked with `wayland-info` on `wayland-0`) both receivers keep their empty defaults, one line
  goes to stderr, and the thread sleeps in `blocking_recv` on the command channel, dropping
  commands, until every `Control` is dropped. The watch senders are held while it parks, so
  receivers read empty values, not a closed channel. The same happens with no socket at all
  (tested), and after the compositor goes away (both channels are emptied first).
- **F22 Commands are gated on what the compositor advertised.** Toplevel commands need the
  manager's matching capability (`close`, `activate`, `minimize`); `Activate` uses the first seat
  and restores a minimized window first; `ActivateWorkspace` needs the workspace's `activate`
  capability, commits, and publishes `WorkspaceEvent::Activated` ahead of the compositor's
  `done`; `MoveWorkspace` needs `zcosmic_workspace_handle_v2` version 2 with the `move`
  capability, moves along axis 0 (COSMIC's groups are one row) and commits. A command for an
  unknown key or a missing capability is dropped.
- **F23 Brief items the frozen interface does not carry.** The brief asks for workspace
  coordinates and capabilities in the published state, pin/unpin, and move-toplevel-to-workspace.
  `Workspace` has no coordinates or capabilities field (coordinates are used for order only) and
  `WlCommand` has no `Pin`, `Unpin` or `MoveToWorkspace` variant; adding either is a frozen-type
  change, so they are reported, not added.
- **F24 Live run facts (nested cosmic-comp 1.8.0, `COSMIC_BACKEND=winit`).** The live test ran
  with `cosmic-term` (`SILL_NESTED_CLIENT="cosmic-term com.system76.CosmicTerm"`): `foot` is not
  installed on this machine. The window appeared 0.7 to 1.2 s after spawn, `Active`, on
  `WorkspaceKey(1)` and output `WINIT-0`; `Minimize`, `Activate` (restored, active, first in MRU)
  and `Close` each took effect. One group, one workspace named `1`, `stable_id` empty
  (cosmic-comp sent no ext-workspace `id`). Two harness facts, both shell-host's
  `scripts/dev/nested.sh`: with the host display in DPMS off, nested cosmic-comp on the NVIDIA
  EGL path blocks in buffer swap and never answers a client's first roundtrip
  (`__GL_SYNC_TO_VBLANK=0` in the environment unblocks it; `SILL_NESTED_GPU=amd` failed with the
  same `EGL_BAD_SURFACE`); and `nested.sh stop` leaves `wayland-1` and `wayland-1.lock` behind,
  so the next `start` reuses the name, sees no new socket, and times out (G6).

## M1 svc-tray (2026-09-24)

- **F29 `system-tray` 0.8.9's item addressing.** `Client`'s public events (`Event::Add`/
  `Update`/`Remove`) name an item by its bus **destination** alone (e.g. `:1.42`), never
  `<destination><path>`; `Client::activate`'s notifier-item proxy always targets the constant
  `/StatusNotifierItem` path too, regardless of where an item actually registered
  (`client.rs::get_notifier_item_proxy`). So every `TrayItemId` here is
  `<destination>/StatusNotifierItem` — both the frozen doc comment's shape and the only path
  `Client::activate` can reach — which is also exactly the shape `model.rs`'s own test fixtures
  already used (`":1.{name}/StatusNotifierItem"`). The `data` feature (default-on) keeps
  `Client`'s own item cache merged with every `Menu`/`MenuDiff` update already applied
  (`data::TrayItemMap::apply_update_event`), so `tray::host` reads the merged `TrayMenu` back
  off `client.items()` on every menu-related `UpdateEvent` rather than tracking diffs itself.
  SNI's `AttentionIcon`, `OverlayIcon` and `Tooltip` updates have no `TrayEvent` variant to land
  in (the frozen model), so they are read and dropped.
- **F30 `TrayCommand` has no `Scroll`.** The brief asks for "activate/secondary-activate/scroll"
  through `Control`; the frozen `TrayCommand` (`model.rs`) has `Activate`, `SecondaryActivate`,
  `MenuPick` and `MenuOpening` only — no `Scroll` variant, and no room to add one without
  changing a frozen enum. `org.kde.StatusNotifierItem.Scroll` is never sent. `SecondaryActivate`
  *is* wired end to end at the service layer, but nothing in the bar UI can trigger it yet: F32.
- **F31 Icon handling in the tray service.** `TrayIcon` (frozen) only ever holds a name or the
  raw ARGB32 bytes, never a URL — a resolved `data:`/`file:` URL is a UI-time read (icon theme
  lookup is I/O; the pixmap's PNG bytes depend on nothing settings-shaped), so it lives in
  `sill_services::tray::icon_url`, not on `TrayIcon` itself. A pixmap is PNG-encoded (the
  pinned `image` crate) and base64'd (hand-rolled — no base64 crate is pinned, matching
  shell-host's own `LocalNet` decoder in spirit) into a `data:image/png;base64,...` URL; a
  named icon is resolved through the installed theme (`freedesktop-icons`, pinned) into a
  `file://` URL. Neither recolours anything: the "symbolic icon recoloured to `--ink`" half of
  the brief needs a live theme token, which a UI-free service must not know, and needs
  rendering it as a Blitz `mask-image` — see F32/Q6, which is why the bar does not call this
  function yet even though it is done and tested.
- **F32 The bar's tray item is a stand-in for per-item icons and can't wire two of the three
  frozen commands.** `ds::Icon` is a closed enum and `IconButton`'s `Glyph` only ever draws one
  of its variants — there is nowhere to put a pixmap's PNG or a theme's SVG, so every item
  renders `Icon::Window` regardless of its real icon (Q6). `ds::MenuEntry::Item` has no
  `children` field (no nested submenus, so a dbusmenu item's `submenu` is dropped) and no
  "disabled" field (so a disabled dbusmenu row renders exactly like an enabled one) — Q7.
  `IconButton`'s `onclick` is `EventHandler<()>`, with no button or pointer-position data, so a
  consumer cannot tell a middle click from a left one, or read where the pointer was — `x, y`
  on `Activate`/`MenuOpening` are `0, 0` (the SNI spec's own words call them "a hint... where to
  show eventual windows (if any)", so this is a real simplification, not a silent one), and
  `SecondaryActivate` is not sent from the bar at all — Q8. `IconButton` also has no `id` prop,
  so the `tray-<n>` popup anchor the freeze asks for sits on a plain, unclassed `span` wrapping
  it (nothing the markup lint checks, since it carries no class) rather than on the button
  itself.
- **F33 The popup's own geometry is provisional.** `bar::popup::menu_popup`/`menu_offset` are
  the only pieces of the bar's popup machinery done at this point; the brief says "see how the
  bar's other menus do it once bar-ui lands" for everything past that, since bar-ui had not
  landed when this was written. The dbusmenu popup is its own `Ds` + `Menu{Dropdown}`, given a
  synthetic `Anchor::Rect` the same width as the popup's own initial `PopupPlacement` size guess
  (`POPUP_WIDTH`, 220px, design/04-COMPONENTS.md section 20's Dropdown width) so the menu's
  right edge lands at the popup surface's own right edge; this composition is expected to be
  revisited once bar-ui's real popup convention exists.
- **F34 Live run facts (this machine, KDE Plasma session).** `cargo test -p sill-services
  --test tray_live -- --ignored --nocapture` found four `StatusNotifierItem`s: "Input Method"
  (`input-keyboard`, 4 menu entries), "Wayland to X11 Video Bridge"
  (`org.kde.xwaylandvideobridge`, Passive, 2 entries), "Toshy Status Indicator"
  (`toshy_app_icon_rainbow`, 20 entries) and "Updates" (`update-high`, 3 entries) — all four
  resolved by icon name, none by pixmap, and every one's dbusmenu layout fetched and flattened
  correctly.
## M1 daemon (2026-09-24)

- **F35 `Services::spawn` calls every module, including ones the M1 wave does not own.** The
  frozen `Services` struct needs a value for every field, and the freeze's own stub comment
  says to call each module's `spawn` (`M4 wires results with the five providers`). `apps`,
  `badges`, `pins`, `launch`, `results`, `wallpaper` and `sound` stay `todo!()` until M2-M6
  land, so `sill run` panics inside `Services::spawn` on whichever of those it reaches first
  until then — expected during M1, not a defect in `Services::spawn` itself. `results` is
  wired with `Vec::new()` for its providers; M4 launcher-core replaces that once
  `sill-services::providers` exists.
- **F36 `on_event`'s frozen signature (`ctx`, `outputs`, one `HostEvent`) cannot reach
  `Services` or the live settings, and spawning a bar needs both.** `daemon::run`'s closure
  (`run_event`, not frozen) intercepts `HostEvent::App(ShellMsg::Ipc{..})` for its own dispatch
  (`Launcher`/`Dock`/`Reload` need `Services` or the settings sender) and, after every other
  event, calls `on_event` and then `spawn_missing_bars`. `on_event` itself does the state-only
  half: output bookkeeping through `outputs::reconcile` (a new output's row is added with every
  surface `None`, which `spawn_missing_bars` reads as "needs one"), closing a removed output's
  surfaces, forgetting a surface the compositor closed, and applying `ShellMsg::Settings` to
  resize the bar (needs only the event's own payload, not `Services`). So `on_event` no longer
  spawns surfaces itself, unlike its doc comment's original "IPC requests are answered";
  `run_event`'s module doc says why. Reported since this is exactly the kind of frozen-signature
  tension CONVENTIONS §1 says to stop and report, even though a workable split was found.
- **F37 `sill dock pin/unpin` is wired to `Services::pins_control`, but cannot run yet.**
  `pins::spawn` is `todo!()` until M2 launch, so the send would need a live pins actor that does
  not exist (F35). `sill reload` re-reads `settings.toml` (real, already implemented) but not
  `dock.json`: `sill_services::pins::load` is also `todo!()`, so pins are left out of Reload
  rather than calling a function that panics; M2 launch should add it once `pins::load` exists.
- **F38 `sill launcher ...` always answers `Failed(Unsupported)`.** No launcher surface exists
  before M4, `Services` has no launcher-visibility control, and `LauncherRequest`'s reply carries
  no way to say "not built yet" versus "failed" other than `Unsupported`, which is the closest
  fit (`FailureKind::Unsupported`'s doc: "this build ... cannot do it").
- **F39 Bar geometry only, on settings change.** `bar.*` is the only settings domain a spawned
  M1 surface reads outside its own component (`bar_layer`'s `size`/`exclusive`); `resize_bars`
  re-applies just those two `SurfaceOp`s per bar on a settled `ShellMsg::Settings`. Every other
  domain's live-reload is the owning surface's own `use_settings()` watch, not the daemon's.
- **F40 `dev/accept-bar.sh` gained four steps beyond the freeze's four**: `single_instance`
  (a second `sill run` exits 0 fast, without touching Wayland), `reload`, `ipc_latency` (a
  normal request answers well under the 2 s reply timeout — nothing reachable from a shell
  script can be made to hang on purpose; `crates/sill/tests/ipc.rs` and `ipc_server`'s own
  tests exercise the real timeout with a fake dispatcher), and `quit` (stops the daemon, so it
  runs last). `exclusive_zone`, `idle` and `menus` stay `exit 2`, naming bar-ui.
- **F41 Client `Quit`/`Ping` never start a daemon** (`client.rs`'s doc comment already said so);
  every other request uses `connect_or_start`. `ClientError` gained `thiserror::Error` (`Debug`
  only in the freeze) so `client::main` can print a real message; the enum's variants are
  unchanged.
- **F42 A lib target was added (`crates/sill/src/lib.rs`) so `crates/sill/tests/ipc.rs` can
  exercise `ipc_server` without a running host.** `main.rs` is now a thin wrapper calling into
  it; no public signature moved, only where it lives. `ipc_server::spawn`/`serve_one` (frozen)
  are built on new, generic `spawn_with`/`serve_one_with`, which take a plain dispatch closure
  instead of a `HostHandle`, so a fake dispatcher drives the same read/timeout/write path the
  real one does — Ping->Pong, an over-long line->`Failed`, and the 2 s timeout are all exercised
  this way, live in `ipc_server.rs`'s own tests and in `tests/ipc.rs`.
- **F43 `sill_settings::watch` needs an entered runtime, and `daemon::run` calls it before
  `Host::run` enters one.** Found by actually running `sill run` against a nested cosmic-comp
  (`dev/nested.sh start`, `WAYLAND_DISPLAY=wayland-1`): it panicked at
  `there is no reactor running, must be called from the context of a Tokio 1.x runtime`
  (`tokio::task::spawn::spawn_inner`, from inside `sill_settings::watch`'s own body). Fixed with
  `let _entered = runtime.enter();` around that one call. Past it, the daemon reaches
  `Host::connect`, spawns the bar's layer surface, and the IPC server answers: `sill ping`,
  `sill run` (single instance), `sill reload` and `sill quit` all round-tripped correctly
  against the live daemon, and `dev/accept-bar.sh` passed `ping`/`single_instance`/`reload`/
  `ipc_latency`/`quit` (in that order, `quit` last since it stops the daemon). `Bar` itself
  still panics — `not yet implemented: M1 bar-ui` at `sill-surfaces/src/surfaces/bar/mod.rs:27`
  — but Dioxus's own render catches it (the panic does not unwind past `initial_build`), so the
  daemon stays up with a broken bar surface rather than crashing; `exclusive_zone`/`idle`/
  `menus` stay `exit 2`, naming bar-ui, since there is nothing to measure yet.

## M1 bar-ui (2026-09-24)

- **F44 One popup, anchored by a measured rect.** Every bar menu is one `xdg_popup` opened with
  `bar::popup::menu_popup`'s config and `PopupAnchor::Rect`, not `Element`: ds `Button` and
  `IconButton` render no `id`, and a hover switch (`SurfaceOp::Reposition`) needs the new
  title's rect in any case. The rect is the button's own, from its `mounted` handle through
  `ds::use_rect`, and travels with each title event to the session. The popup is a document of
  its own (`menu_popup.rs`) that follows a per-popup `watch` of its view: a new view swaps the
  contents and repositions, `None` makes it close itself (a `SurfaceHandle` closes only its own
  surface). It reports picks, dismissals and its own drop on an `mpsc` the bar drains into the
  tracking machine (`plan::popup_input`).
- **F45 The popup carries its own room.** A ds floating surface is clamped 8 px inside its
  bounds (`ds::place`), so the popup's `Ds` root sits in a frame inset to leave the menu 24 px
  either side, 8 above and 40 below (F53); the anchor rect folds that room in and
  `PopupConfig::offset` carries design/13 §13.3.2's (-8, +6) / (+8, +6) (G3, closed upstream).
  The size is an estimate from the entries (G7); the spare area is transparent.
- **F46 What the machine drives, and what the ds Menu keeps.** The machine gets presses,
  releases and pointer entries on titles from the bar, and picks, Escape/outside-margin closes
  and drops from the popup; `Open`, `Switch`, `Close` and `Pick` become show, swap, hide and run.
  `Highlight`, submenus, the safe triangle and `Adjacent` are not wired: the ds `Menu` draws its
  own selection from the pointer and keys and exposes neither item hover nor item release (Q11),
  and bar menus have no submenus. Press-drag-release onto an item therefore needs a click on the
  item when the compositor keeps the implicit grab on the bar. A press on the bar outside every
  title is an outside press (as on macOS); a real outside press ends the grab and the popup's
  drop reports it.
- **F47 Status lines are menu headers.** ds `MenuEntry` has no disabled or informational item
  (its TODO O-1), so network, battery and clock menus show their lines as headers and end with a
  Settings item (`LaunchCommand::Launch` of `com.system76.CosmicSettings`). The volume menu is a
  ds `Popover` with a `SectionHeader{Field}`, a `Slider` and a mute `Toggle`, since a `Menu`
  cannot hold a slider.
- **F48 The bar's tint.** `Ds` paints frame layers (the A/B cross-fade) only for
  `Material::Window` (Q9). The bar paints its own layer of `--f-solid` at `--m-tint-alpha` over
  the Bar material, with `background-color` and `color` transitions over `--t-scene`, so a
  workspace switch cross-fades the hue (design/21 §5 through spike S4) but the layer is the
  Space's first stop, not its gradient.
- **F49 Keys that do not reach quire components.** `bar.status_icon_box_px`,
  `bar.status_glyph_px` and `bar.glyph_size_policy` have no effect: `IconButton{Tool}` is 28 x 26
  with a 16 px glyph and a consumer may not style `.ds-*` (Q12). `bar.open_title_pill_height_px`
  is the title's hit height (both 24 by default): the pill is the hit box's background. The app
  name is a `Button{Quiet}`, so it is not ui 13/700. `bar.height_px`, `title_hit_height_px`,
  `title_padding_px` and `status_gap_px` are written inline from the settings.
- **F50 Not built in this wave.** The workspace pills' right-click Edit Space (the Space editor
  popup is later work); the bar menus' popup blur (`BlurState::Unavailable`, the solid tint:
  a popup's document would have to name the menu element for a blur region, and ds `Menu`
  renders no id); the open and close animations design/13 asks for (the ds `Menu` always plays
  its entrance, and the popup goes at once on close).
- **F51 `use_space_look`** returns `ds::default_look(index, spaces.default_grain,
  spaces.default_card_accent)`: design/21 §4's preset for the workspace's position in its
  group (F17). With no workspaces (KWin) the bar takes preset 1.
- **F52 Acceptance.** `dev/accept-bar-ui.sh` (nested `kwin_wayland --virtual`, 1280 x 720
  logical at scale 1.5, `examples/bar_demo.rs` on fake services): a maximised probe is 720 tall
  without the bar and 688 with it (32 reserved); five idle seconds add 0 frames, 0 commits and
  0 callbacks; a click on the clock opens `sill-bar-Virtual-0/popup` (268 x 127, presented) and a
  click on the bar's empty middle closes it; the popup's capture is transparent (alpha 0) at its
  corners, above the card and beside its top-right corner. `dev/accept-bar.sh` on the nested
  cosmic-comp (2176 x 1360, Mesa) with `sill run` (debug build): every step passes, with a
  maximised probe of 1328 = 1360 - 32, the daemon's `sill-bar-WINIT-0` at 8 frames and 8 commits
  before and after five idle seconds, and the menus step through `accept-bar-ui.sh cosmic`
  (`sill debug` has no click, G8), whose screenshot shows `bar_demo`'s bar under the daemon's.
  Screenshots: `dev/out/bar.png` (KWin, clock menu open), `dev/out/bar-cosmic.png` (the daemon's
  bar with the live clock, tray, volume and network), `dev/out/bar-cosmic-menu.png`.
- **F53 The popup's shadow room and root.** The popup keeps 24 px either side, 8 above and 40
  below its menu for `--shadow-pop`, by placing its `Ds` root in a frame of its own (the overlay
  bounds `ds::place` clamps to), and its root is `Material::Bar`, not `Popover`: a root that
  holds only overlays is a zero-height, full-width box, and Popover's drop shadow on it painted a
  grey band across the popup's top (Q13). The menu card's own paint does not depend on the
  root's material.

### Gaps found in M1 bar-ui

- **G7** `PopupSize::FitContent` never measures the popup's document: the size passed in
  `PopupPlacement` is the size (F45).
- **Q9** `Ds` renders the frame layers only for `Material::Window`; the tinted shell materials
  (Bar, Dock) need the same A/B gradient layers at the tint alpha (design/21 §3, §5) (F48).
- **Q10** quire's `HostMeasure` for Blitz documents is private to ds-native, and shell-host cannot
  provide one, so every shell-host root without one panics ("RefCell already borrowed") on its
  first `use_rect` read once shell-host polls woken tasks inside the render (shell-host
  `df9c59b`). `bar/measure.rs` repeats ds-native's twelve lines; ds-native should export it.
- **Q11** ds `Menu` exposes no item hover or item release, and calls `onclose` before `onpick`, so
  an external tracker (design/13 §13.7) cannot drive its highlight, submenus or
  press-drag-release (F46); nor can its entrance be suppressed for a hover switch (F50).
- **Q13** A `Ds` root (or `Surface`) paints its material's box shadow on its own box even when
  it holds nothing but overlays, so a popup document gets a shadow band across its top (F53).
  A host-only root (no material paint) would let a popup keep `Material::Popover`.
- **G8** sill's frozen `DebugRequest` has `Pointer` (a motion) but no click, while shell-host's
  `HostCtx::inject` has `Click`: `sill debug` cannot open a bar menu, so `dev/accept-bar.sh`
  checks menus through `bar_demo` (F52).
- **Q12** No ds status-item variant: `IconButton{Tool}` cannot take the bar's glyph box and size
  keys, and ds components on frame chrome use paper inks, not `--f-ink*` (F49). No wired-network
  glyph (`Icon::Link` stands in).

## Gaps reported upstream

- **Q1** quire has no menu tracking machine (F15).
- **Q2** quire has no easing evaluator for `CubicBezier` (F16).
- **Q3** quire has no `spaces.json` store (F17).
- **Q4** ds-settings' file I/O and watch are not generic over the settings file (F18).
- **Q5** The schema derive: text only by the literal type `String`; a one-variant enum cannot
  be described; a numeric newtype without an explicit `range` is read as an enum (a compile
  error naming `SchemaVariants`, not a message about the missing range).
- **Q6** No `ds` component can draw an externally-sourced icon — a pixmap's PNG bytes or an
  icon theme's SVG, as a status tray item or any other third-party icon needs. `ds::Icon` is a
  closed enum of built-in glyphs; `IconButton`'s `Glyph` only ever draws one of its variants.
  design/08-ICONS.md section 1.5 already specifies the mechanics for exactly this case
  ("Blitz: the recoloured tray icon is a `mask-image` data URI with `background: currentColor`,
  the same path as the S6 fallback"), settled as a rule with "proposed mechanics" — nothing
  implements it yet (F32).
- **Q7** `ds::MenuEntry::Item` has no `children` field (so `Menu` cannot draw a submenu) and no
  "disabled"/`enabled` field (so a menu cannot show, or skip, a disabled item) (F32).
- **Q8** `IconButton` has no `id` prop (nowhere to put an anchor id for a popup to open against)
  and its `onclick` is `EventHandler<()>`, carrying no mouse button or pointer position — a
  consumer cannot tell a secondary click from a primary one, or read where the click landed
  (F32).
- **G1** Debug injection: `inject::apply` (feature `debug-inject`) prints stats to stdout
  instead of returning them, and finds a surface by name only inside that module. sill's
  `debug stats` uses `HostCtx::stats`; `pointer`/`leave`/`capture` need sill's `debug` feature.
- **G1** RESOLVED (shell-host `df9c59b`, "Close sill's host gaps G1, G3 and G5"). Was: debug
  injection (`inject::apply`, feature `debug-inject`) printed stats to stdout instead of
  returning them, and found a surface by name only inside that module. Now: `HostCtx::inject(id,
  Inject) -> Result<Injected>` (feature `debug-inject`) delivers pointer/leave/capture and
  returns the result directly (`Injected::Captured{path, size}` for a capture, so success and
  size are visible to the caller, not just a stdout line); `HostCtx::surface_named(&SurfaceName)
  -> Option<SurfaceId>` and `HostCtx::{stats,take_stats}` are unconditional (no `debug-inject`
  needed). `crates/sill/src/daemon.rs` (`dispatch_debug`, `act`, `apply_injection`,
  `collect_stats`) uses all four: `Stats` through `take_stats` (draining, one report per call);
  `Pointer`/`Leave`/`Capture` through `surface_named` + `inject`, gated on sill's own `debug`
  feature (`Cargo.toml`: `debug = ["shell-host/debug-inject"]`); no stdout bridge anywhere.
- **G2** Minimize rectangles (design/10 §10.3.10): cctk's `set_rectangle` needs the dock's
  `wl_surface`, which lives on shell-host's connection, while the toplevel manager lives on the
  `cosmic_wl` thread's own connection. A protocol object cannot cross connections, so the
  minimize target cannot be set until shell-host lets a client bind extra globals on its
  connection (or exposes the manager). `WlCommand` has no such command.
- **G3** `PopupConfig` has no offset (`xdg_positioner.set_offset`); design/13 §13.3.2 places
  bar menus at (-8, +6) from the title. `bar::popup::menu_offset` names the offset for the
  component to approximate by inflating its anchor element.
- **G4** Drag and drop into shell surfaces is not supported (`handlers/data_device.rs`): file
  drops on dock tiles and spring-loading (design/10 §10.3.8) have machine support
  (`DndMotion`, `Drop`, `Spring`) and no event source.
- **G5** No frame-callback hook for documents: the dock machine's `Frame` (a `wl_surface.frame`
  callback, design/10 §10.4) is not observable from a component. Until shell-host exposes one,
  the component drives `Frame` from a timer at the output's refresh while the machine has asked
  for frames (`RequestFrames` .. `StopFrames`), which keeps idle at zero wakeups but is not
  callback-exact.
- **G6** `scripts/dev/nested.sh`: `stop` leaves the nested socket and its lock file in
  `$XDG_RUNTIME_DIR`, which makes the next `start` time out (it waits for a socket name it has not
  seen before); and with the host display in DPMS off the NVIDIA EGL path needs
  `__GL_SYNC_TO_VBLANK=0` or the compositor never answers a client (F24).

## M2 apps-index (2026-09-24)

Filled `apps::{scan, icons}` and `apps::spawn`; `Services::spawn`'s apps placeholder now calls
it. `match_app`, `expand_exec`, `reduce_apps` and the index type were already frozen and
untouched.

- **F66 An absolute `Icon=` is used as-is, never checked against the filesystem.** The
  pre-existing `tests/skeleton.rs::a_desktop_entry_parses_into_an_app` (ignored for this wave,
  now filled and passing) asserts `Icon=/usr/share/icons/foot.svg` becomes
  `AppIcon::File("/usr/share/icons/foot.svg")` even though that file does not exist on the
  machine running the test — i.e. `resolve_icon` takes an absolute path on trust, the same way
  `parse_entry` takes every other field on trust from the `.desktop` file, matching
  `freedesktop_desktop_entry::IconSource::from_unknown`'s own comment ("a name or an absolute
  path") and `AppIcon::File`'s doc ("an absolute `Icon=`, or the theme lookup's answer"). An
  earlier draft added an `is_file()` check "for safety"; the frozen test caught it.
- **F67 `TryExec` excludes the whole entry, not just a `parse_entry` doc-comment case.** The
  brief's visibility list ("NoDisplay, Hidden, OnlyShowIn/NotShowIn ..., TryExec present") and
  the Desktop Entry Specification's own words ("if [not found], the entry may be ignored") both
  point at exclusion, and `App` has no field to represent "not really installed" short of not
  yielding it — so `parse_entry` returns `None` when `TryExec` is set and neither an absolute
  path nor a bare name on `$PATH` resolves to a file. `NoDisplay`, `Hidden` and an
  `OnlyShowIn`/`NotShowIn` mismatch are different: they fold into `Listing::Hidden` (the frozen
  `App::listing`'s own doc, "hidden entries still match windows, never list") and the entry is
  still yielded and still occupies its dedup slot, so a `Hidden=true` override in a
  higher-precedence directory still shadows a real entry lower down.
- **F68 `OnlyShowIn`/`NotShowIn` are matched against a fixed `COSMIC`, not the live
  `XDG_CURRENT_DESKTOP`.** sill only ever runs as the COSMIC session; reading the constant
  keeps the scan deterministic under a nested test compositor or a dev shell where that
  variable is unset or wrong, and matches the brief's own wording ("against
  `XDG_CURRENT_DESKTOP=COSMIC`") read as naming the value, not the mechanism.
- **F69 A trailing `;` in a list field leaves an empty element.** `MimeType=a;b;` (the spec's
  own recommended, semicolon-terminated form) becomes `["a", "b", ""]` through
  `freedesktop_desktop_entry`'s `str::split(';')`; `owned_list` (`MimeType`) and the `keywords`
  mapping both filter it out. `only_show_in`/`not_show_in` need no such filter — an empty
  string never equals `"COSMIC"` — and `Actions=a;b;`'s trailing empty action id is already
  dropped by `actions`'s `filter_map` (no `[Desktop Action ]` group exists for it).
- **F70 The icon theme is read straight from `cosmic-config`'s file layout, not the
  `cosmic-config` crate.** `cosmic-config` is not in the pinned dependency block (`docs/workspace-deps.toml`),
  and the value is one line (`~/.config/cosmic/com.system76.CosmicTk/v1/icon_theme`, a quoted
  RON string) — reading and unquoting it directly avoids adding a dependency for one file read.
  `icon_theme()` falls back to `"Cosmic"` when the file is absent, empty or unreadable (no
  `com.system76.CosmicTk` config exists on the dev machine); `freedesktop_icons::lookup`'s own
  fallback to `hicolor` when a theme is not installed covers the plan's third step, so this
  function only ever chooses between the first two.
- **F71 SVG-vs-largest-PNG preference is a heuristic, not exact, given what `freedesktop_icons`
  0.4.0 exposes.** `try_get_icon`'s directory search returns the *first* directory, in the
  theme's own `index.theme` section order, that has *any* matching file at the requested size —
  not the first SVG across every matching directory. `resolve_icon` asks once with
  `force_svg()` at `MAX_SIZE` (256) and keeps the result only if it actually is an `.svg` file
  (checked by extension, since `force_svg` can still fall back to a raster inside whichever
  directory it lands on first), then falls back to a plain lookup at the same size. This is
  exactly right for every theme where the scalable directory is declared before (or matches
  more narrowly than) the fixed ones — hicolor, Adwaita, Papirus, Cosmic and every theme found
  on the live run all are — but a theme whose `index.theme` lists a fixed directory before its
  scalable one at the very requested size could still surface that PNG instead of the SVG.
  Fixing this exactly would mean re-implementing the crate's own directory-selection and
  candidate-scoring logic; not done, since no installed theme case exercises it.
  `tests/icons.rs` proves the intended ordering against a purpose-built fixture theme.
- **F72 The icon-theme fixture test cannot use `std::env::set_var`, and writes into a real
  `$HOME/.icons` subdirectory instead.** `freedesktop_icons::THEMES` is a process-wide
  `once_cell::sync::Lazy`, seeded once from `$HOME`/XDG env at first use with no API to point a
  lookup at an arbitrary directory; the only way to steer it is the environment read at that
  first use. `std::env::set_var`/`remove_var` are `unsafe fn` since edition 2024, and this
  workspace denies `unsafe_code` everywhere but shell-host's one allowed module (CONVENTIONS
  §11) — sill-services may carry none at all. So `tests/icons.rs` (a lone `#[test]`, its own
  process, nothing else in that binary ever touching `THEMES` first) creates its fixture theme
  under the real `$HOME/.icons` (one of `freedesktop_icons`' own unconditional, `.exists()`-
  filtered base paths) with a name unique per process and counter, and a `Drop` guard removes
  only that one subdirectory afterward, panic or not. It is the one place in this crate's tests
  that touches the real home directory.
- **F73 `tempfile` is not in the pinned block; a hand-rolled `TempDir`.** Same shape and same
  reason as `sill-settings/tests/common::TempDir` (its own comment: "no `tempfile`: not in the
  pinned block"); `apps/icons.rs`'s unit tests and `tests/icons.rs` each carry their own small
  copy rather than sharing a `tests/common` module across a src-level unit test and an
  integration test (they cannot share one: `#[cfg(test)] mod tests` in `src/` cannot reach
  `tests/common`).
- **F74 Flatpak's export directories are added even when `XDG_DATA_DIRS` does not already carry
  them.** Confirmed necessary, not just defensive: the live run's own `XDG_DATA_DIRS` did not
  include `/var/lib/flatpak/exports/share` (no `/etc/profile.d/flatpak.sh` sourced in this
  shell), so `applications_dirs`' safety net was the only reason
  `/var/lib/flatpak/exports/share/applications` was searched at all.
- **F75 Live run facts (this machine, KDE Plasma/Toshy session, no COSMIC installed).**
  `cargo test -p sill-services --test apps_live -- --ignored --nocapture`: 353 applications
  found across `~/.local/share/applications`, `~/.local/share/flatpak/exports/share/applications`,
  `/var/lib/flatpak/exports/share/applications`, `/usr/local/share/applications` and
  `/usr/share/applications`; icon theme resolved to `Cosmic` (no `com.system76.CosmicTk` config
  on this machine, F70's fallback). Five sample entries, icons resolved by both routes:
  `Toshy_Tray` (svg, `.../hicolor/scalable/apps/toshy_app_icon_rainbow.svg`), `app.toshy.preferences`
  (same svg), `claude-code-url-handler` (`Listing::Hidden`, `NoDisplay`, `icon=None`),
  `line-messenger` (png, `.../hicolor/256x256/apps/line-messenger.png`), `microsoft-teams`
  (same, 256px png). No crash, no entry that failed to parse silently taking down the scan.

## M1 svc-dbus (2026-09-24)

Filled `battery::spawn`, `network::spawn` and `volume::spawn`. All three signatures were usable
as frozen; nothing there needed to change.

- **F25 Battery via upower_dbus's `DeviceProxy`.** `UPowerProxy::get_display_device()` gives the
  display device; its five properties (`IsPresent`, `Percentage`, `State`, `TimeToEmpty`,
  `TimeToFull`) are read once to seed, then followed via `receive_<property>_changed()` streams
  polled with a small `poll_fn`-based `next()` helper (zbus's `PropertyStream` needs `futures_core`
  for `.poll_next()`, not `.next()`; rather than add a `futures-util`/`tokio-stream` dependency to
  the pinned block, `zbus::export::futures_core` — already re-exported, if `#[doc(hidden)]` — is
  enough). On this desktop machine (no battery) it seeds `Battery { presence: Absent, .. }` inside
  the first poll, which is the documented absent value.
- **F26 Network via cosmic-dbus-networkmanager's raw interface proxies.** `NetworkManagerProxy`
  gives `PrimaryConnection`/`Connectivity`; the primary path is resolved to an
  `ActiveConnectionProxy` for `Type`/`State`, and for a Wi-Fi type, its first device is tried as a
  `WirelessDeviceProxy` for `ActiveAccessPoint`, then an `AccessPointProxy` for `Ssid`/`Strength`.
  Every step that can fail (wrong device kind, no AP, disassociated) falls back to `Link::Other`
  rather than propagating an error, since a partial read of a real connection is still a real
  connection. Live updates come from `PrimaryConnection`/`Connectivity` property streams only: an
  access point's own `Strength` can change without the primary connection changing, and streaming
  that too would mean re-subscribing on every reassociation; out of scope for M1, left for a
  later wave if continuous RSSI turns out to matter for the bar item. On this machine it seeds
  `Network { link: Wifi { ssid: "home-wifi", strength: Percent(70..72) }, connectivity: Full }`.
- **F27 Volume via libpulse-binding's *standard* mainloop, not the threaded one the plan names.**
  The threaded mainloop's callbacks (state, subscribe) are invoked by a second, C-spawned thread;
  the crate's own canonical usage reaches back into the shared `Context`/`Mainloop` from inside
  those callbacks via `RefCell::as_ptr()` plus a raw-pointer deref (its own doctest does exactly
  this), because a `Ref`/`RefMut` guard held across the FFI call that blocks waiting for that
  second thread is a genuine cross-thread data race on `RefCell`'s own (non-atomic) borrow flag —
  confirmed by hand: the first attempt, built exactly to that pattern with `Rc<RefCell<_>>` and
  `mainloop.lock()`/`wait()`/`signal()`, deadlocked every real run (never printed, never returned,
  0% CPU, `Sl` in `ps`). `unsafe` is denied workspace-wide outside shell-host's one allowed module
  (CONVENTIONS §11), so the threaded mainloop cannot be used soundly from here. `volume::spawn`
  instead runs `mainloop::standard::Mainloop` (no thread of its own) on the dedicated OS thread
  `Handle::spawn_blocking` already gives it, polling `iterate(false)` and `commands.try_recv()` in
  a 10ms loop. Every callback then runs on that same one thread, so `Rc<RefCell<Context>>` is
  sound with no unsafe at all, and DESIGN.md's "the `cosmic_wl` and volume threads on their own
  threads" still holds — it is still one dedicated thread per service, just not the specific
  libpulse primitive named "threaded". Subscribing to `SINK | SERVER` and re-fetching
  `get_server_info` → `get_sink_info_by_name` on every event (rather than tracking the sink index
  and patching in place) keeps the reducer's `Sink { .. }` event as the only event this service
  ever sends for a level/mute/description change, at the cost of one extra round trip per event;
  fine for a bar item. On this machine it seeds `Volume { presence: Present, level: Percent(50),
  mute: Audible, sink: "USB Audio S/PDIF Output" }`.
- **F28 A `match` scrutinee keeps its `Ref` borrowed for every arm.** `match context.borrow().get_state() { Ready => { ready = true; subscribe(&context, ..); refresh(&context, ..); } .. }`
  panicked ("already borrowed") the first time `subscribe` tried its own `context.borrow_mut()`,
  because Rust extends a `match` scrutinee's temporaries across the whole match, arm bodies
  included — the classic `match mutex.lock().foo() { .. }` footgun, just with a `RefCell` instead
  of a `Mutex`. Fixed by binding `let state = context.borrow().get_state();` first, so the `Ref`
  drops at the `;` before the `match` (and the calls inside its arm) ever runs.

## M2 badges (2026-09-24)

Filled `badges::spawn` (`crates/sill-services/src/badges/live.rs`, new). The frozen shapes
(`Badges`, `Badge`, `BadgeProp`, `BadgeEvent`, `reduce_badges`, `parse_app_uri`) needed no
change.

- **F54 A bus-wide signal subscription needs `MessageStream::for_match_rule`, not a generated
  proxy.** `com.canonical.Unity.LauncherEntry` has no crate binding (unlike UPower or
  NetworkManager) and, unlike `StatusNotifierItem`, an app never registers a well-known name to
  publish it — it just broadcasts `Update` from whatever connection it already has. zbus 5's
  `MessageStream::for_match_rule(rule, &connection, None)` registers a raw `MatchRule` (`type=
  'signal', interface='com.canonical.Unity.LauncherEntry', member='Update'`, no sender) with the
  bus and yields every matching `Message` regardless of who sent it; `Body::deserialize::<(String,
  HashMap<String, OwnedValue>)>()` decodes the `(s app_uri, a{sv})` body. No new dependency: zbus
  was already in the pinned block for `battery`/`network`/`tray`/`volume`.
- **F55 Leniency is a per-key decode, not a per-message one.** The signature `(s, a{sv})` is
  fixed by the interface, so a message that isn't that shape at all (or whose `app_uri` names no
  app) is simply dropped — there is no partial reading of a tuple. What *is* partial is each
  value inside the dictionary: `decode_prop` downcasts by key (`count` -> `i64::try_from`,
  `progress` -> `f64::try_from`, the three booleans -> `bool::try_from`) and drops only the key
  whose value has the wrong `zvariant::Value` variant, via `filter_map` rather than a `Result`
  that would fail the whole `Update`. `reduce_badges` already treats an absent `BadgeProp` as
  "unchanged" (model.rs, done at W0), so a dropped key and a key the sender never sent behave
  identically — no special case needed on the decode side for "missing".
- **F56 Which sender is publishing an app's badge is this task's own bookkeeping, not part of
  `Badges`.** The frozen `Badges` only ever remembers `app -> Badge` (the plan's persisted
  shape), so there is nowhere in it to record which connection last spoke for an app. `live::run`
  keeps a local `HashMap<AppId, OwnedUniqueName>` alongside the loop, updated on every `Update`
  and consulted on `NameOwnerChanged`: a name-owner change is only a clear when `new_owner` is
  empty (a name *transfer* between two still-live connections is not a departure) and its `name`
  matches a tracked unique name.
- **F57 Verification.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --features sill/debug`,
  `./scripts/check-boundary.sh` and `cargo deny check licenses` all pass. The `#[ignore]`d live
  test (`tests/badges_live.rs`) was run once against this machine's real session bus (`cargo test
  -p sill-services --test badges_live -- --ignored --nocapture`): it starts `badges::spawn`,
  spawns `examples/badge_emitter.rs` as a child publishing `org.sill.BadgeEmitterTest` with
  `count = 7`, asserts the badge appears within the 1 s budget, kills the emitter, and asserts the
  badge clears within the same budget — both passed (finished in 0.11 s total, well inside
  budget).
## M2 launch (2026-09-24)

Filled `launch::{spawn, launch_argv, scope_unit}`, `pins::{spawn, load, save}` and
`crates/sill/src/launch.rs`; wired both into `services.rs`'s two placeholder lines.

- **F58 The terminal for `Terminal=true`.** `$TERMINAL` (trimmed, non-blank) then ghostty,
  wezterm, foot, cosmic-term, the first found as an executable file on `$PATH`
  (`launch/terminal.rs`); wrapped `[bin, "-e"]`. No trace exists for wezterm's or cosmic-term's
  own flag (CONVENTIONS "a quirk without a trace is a rumour"), so `-e` is a considered default,
  not a verified one — worth a second look once one of them is actually exercised. On this
  machine: `$TERMINAL` is unset, ghostty is on `PATH` (the user's own preference, per the brief)
  and is chosen; wezterm and foot are absent; cosmic-term is present but ghostty sorts first.
- **F59 Pins live under `dirs::config_dir()`, not `sill_settings::settings_dir()`.**
  `sill-services` may not depend on `ds-settings`/`sill-settings` (boundary: F2, and
  `scripts/check-boundary.sh`'s `sill-services` rule), so `pins::pins_dir` calls the already-pinned
  `dirs` crate directly (`dirs::config_dir().join("sill")`) rather than sharing
  `sill_settings::settings_dir`'s helper — both resolve to the same `$XDG_CONFIG_HOME/sill` in
  practice, but they are two independent readings of XDG, not one shared function (Q4 already
  covers the general shape of this duplication for `settings.toml`; this is its `dock.json`
  sibling). `pins::spawn` degrades to in-memory-only (a stderr line, no watch, no save) when
  neither `$XDG_CONFIG_HOME` nor `$HOME` is set, the same shape `ServiceError`'s doc promises for
  every other service's missing dependency.
- **F60 Unit-name escaping.** `scope_unit` escapes an app id the way `systemd-escape` escapes a
  unit name component, from general knowledge of systemd's own behaviour rather than by reading
  `unit-name.c` (systemd is LGPL/GPL; CONVENTIONS "note-and-close" and "never put GPL … source
  into a model's context"): `/` becomes `-`; a leading `.` and every byte outside
  `[A-Za-z0-9:_.]` — a literal `-` included, since `-` is reserved for an escaped `/` and would
  otherwise be ambiguous on unescape — becomes `\xHH`. Table-tested against the cases that matter
  here (reverse-DNS ids, a literal dash, a leading dot, a space); not cross-checked against a real
  `systemd-escape` binary output for every byte value, since sill's own app ids never contain most
  of them.
- **F61 `%i`'s value is the resolved icon file, not the entry's own `Icon=` name.** `App` (frozen,
  `apps/index.rs`) keeps only the theme-resolved `AppIcon` (`File(PathBuf)` or `None`), not the
  raw `Icon=` string a `.desktop` file names — apps-index's scan already threw that away. So
  `launch_argv`'s `ExecArgs.icon` is the resolved file's path when there is one; an app whose
  `Exec` uses `%i` gets `--icon /path/to/theme/resolved/icon.png` rather than `--icon the-name`.
  Neither the plan nor design/10 says which `%i` should carry; this is a reasonable reading, not
  a verified one, and it is the only reading `App`'s frozen shape allows without re-reading the
  desktop file at launch time.
- **F62 A named action `LaunchRequest` no longer has falls back to the entry's own `Exec`.**
  `ExecError` (frozen, `apps/exec.rs`) has no "unknown action" variant, so `action_exec` cannot
  refuse the launch when `DesktopAction::Named(id)` names an action `app.actions` does not have
  (a stale index between the launcher and the app scan, in principle); it launches the entry's
  default `Exec` instead of failing outright. Table-tested (`a_named_action_replaces_exec_and_an_
  unknown_one_falls_back`).
- **F63 The process-group fallback triggers on `ErrorKind::NotFound` from starting `systemd-run`
  itself**, not on any failure inside the scope: `launch::run::start` tries the full
  `systemd-run --user --scope …` argv first, and only when the very first exec (`systemd-run`
  is not on `PATH`, or — the same error from `posix_spawn` — no user systemd manager answers)
  does it fall back to the bare command (the part of `argv` after `systemd-run`'s own `--`) in a
  fresh process group (`Command::process_group(0)`, plan "Launch": "fallback process group").
  Either path hands back the same `tokio::process::Child` type, and `wait_for_exit` does not care
  which one ran — `systemd-run --scope` execs in place of itself, so the tracked `Child` *is* the
  app either way. Table-tested by aiming `start` at a made-up binary name rather than by hiding
  the real `systemd-run` (`launch::run::tests::a_program_that_cannot_start_falls_back_to_the_
  bare_command`).
- **F64 Live run facts (this machine, user systemd present).** `cargo test -p sill-services --lib
  a_launch_runs_in_a_real_systemd_scope_and_exits -- --ignored --nocapture`: `/bin/true` through
  the real `systemd-run --user --scope --collect` path reported `Started` with unit
  `app-sill-sill\x2dlaunch\x2dtest-1.scope` (the `AppId`'s two literal dashes escaped, as F60
  says they should be), `systemd-run` printed its own `Running as unit: …; invocation ID: …`
  line to the test's stderr, and the same unit was reported `Exited` once the process finished —
  both within the test's timeouts, in well under a second in practice.
- **G9** `crates/sill/src/launch.rs::launch` has no unit test: `shell_host::SurfaceHandle::new`
  is `pub(crate)` to shell-host, so nothing outside that crate can construct one, even a
  throwaway, for a test. The function is two lines against two already-frozen, already-tested
  signatures (`SurfaceHandle::activation_token`, `LaunchCommand::Launch`); it is exercised live
  once dock clicks call it (design/10 §10.3.6, dock-ui). A `SurfaceHandle` test constructor (or a
  trait shell-host tests already use) would close this gap; reported rather than added here since
  `shell-host` is not owned by this wave.
- **F65 `services.rs`'s own `spawn_does_not_panic_and_every_stub_holds_its_default` test reaches
  the real `~/.config/sill` directory** now that `pins::spawn` is real (F59): it asserts
  `services.pins.borrow() == DockPins::default()`, which only holds because this machine had no
  `~/.config/sill/dock.json` when the suite ran. The same test already reaches the real session
  and system buses for `battery`/`network`/`volume`/`tray` (F25-F27, F29-F34) and the real
  Wayland socket for `cosmic_wl` (F20-F24), so a real (empty) XDG config directory is the same
  kind of live dependency this test has always had, not a new one; noted here since it is now
  true of `sill-services`' own filesystem too, not only of buses and sockets.
## M2 freeze amendments (2026-09-24)

- **F76 The freeze was amended at the M1→M2 boundary.** CONVENTIONS §1 makes a frozen
  signature a stop-and-report, and the M1 wave did stop and report: `Workspace` had no
  coordinates or capabilities and `WlCommand` no pin, unpin or move-to-workspace (F23),
  `TrayCommand` had no `Scroll` (F30), `DebugRequest` had no click so `sill debug` could not
  open a menu (G8), and every shell-host root needed a `ds::HostMeasure` that only the bar
  supplied (Q10). The dock (design/10 §10.3.6-10.3.7) and the switcher (design/13 §13.3.5) are
  the next consumers of the same types, so the reported items were added in one coordinated
  edit on `m2-amend`, while no M2 agent was coding against them, rather than as a surprise in
  the middle of M2. Every change is additive and every existing test is unchanged:
  - `sill_services::Workspace` gains `coordinates: Vec<u32>`, `capabilities: WorkspaceCaps`
    and `pinning: Pinning`, all `#[serde(default)]` (F77); new public `WorkspaceCap`,
    `WorkspaceCaps`, `Pinning`.
  - `sill_services::WlCommand` gains `PinWorkspace(WorkspaceKey)`,
    `UnpinWorkspace(WorkspaceKey)` and `MoveToplevelToWorkspace { toplevel: WindowKey,
    workspace: WorkspaceKey }` (F78). `SetMinimizeRect` is **not** added: `set_rectangle`
    needs the dock's `wl_surface`, which lives on shell-host's connection, and shell-host's
    G2 (a way to bind the toplevel manager on its connection) is designed but not built.
  - `sill_services::TrayCommand` gains `Scroll { item: TrayItemId, delta: ScrollDelta }`, with
    new public `ScrollDelta { axis: ScrollAxis, steps: i32 }` and
    `ScrollAxis::{Vertical, Horizontal}` (F79).
  - `sill_ipc::DebugRequest` gains `Click { surface: SurfaceName, at: Point, button:
    PointerButton }`, new public `PointerButton::{Primary, Secondary, Middle}`; the CLI gains
    `sill debug click <surface> <x> <y> [--button primary|secondary|middle]` (F80).
  - `sill_surfaces::MEASURE` (new `sill-surfaces/src/measure.rs`) replaces the bar's private
    `surfaces/bar/measure.rs`, and the daemon provides it on every root (F81).
- **F77 Workspace coordinates, capabilities and pinning.** `describe::workspace` copies cctk's
  `coordinates` as they are (the groups were already ordered by them, F20) and merges the
  ext-workspace capability bits (`activate`, `deactivate`, `remove`, `assign`) and COSMIC's
  (`rename`, `set_tiling_state`, `pin`, `move`) into one `WorkspaceCaps`: a set that sorts and
  deduplicates however it is built or decoded, so equal sets compare equal, with `contains`
  for the gates. COSMIC's protocol XML gives `pin` the bitfield **value 3**, which is
  `rename | set_tiling_state`; every flag is therefore compared whole (`contains`), and a
  compositor that offers rename and tiling but not pin reads as offering pin. That is the
  protocol's spelling, not ours; the table test pins it. `pinning` was not in the M1 report
  but pin and unpin are useless to a surface that cannot show which one applies, so it rides
  the same amendment, from COSMIC's `state` event (`pinned`), `Unpinned` where there is none.
  All three fields are `#[serde(default)]`: the defaults (no coordinates, no capabilities,
  unpinned) are what a compositor without them says, and a line of the old shape still decodes
  (`a_workspace_without_the_amended_fields_still_decodes`).
- **F78 The new commands are gated the way the existing ones are.** `PinWorkspace` and
  `UnpinWorkspace` need a `zcosmic_workspace_handle_v2` of version 2 advertising `pin`, send
  `pin`/`unpin` and commit the ext-workspace manager; `MoveWorkspace` now goes through the
  same `cosmic_offers(version, offered, wanted)` gate (table-tested). `MoveToplevelToWorkspace`
  needs the toplevel manager at version 4 advertising `move_to_ext_workspace` **or** the older
  `move_to_workspace`: nested cosmic-comp 1.8 binds the manager at version 4 and carries out
  `move_to_ext_workspace`, yet its `capabilities` event lists only `close, activate, maximize,
  minimize, move_to_workspace` (seen live; the older request is a no-op there, and it takes a
  `zcosmic_workspace_handle_v1` this thread never has). Gating on the ext capability alone
  dropped every move on the one compositor that implements it (table-tested:
  `move_to_ext_workspace_is_gated_on_version_4_and_either_capability`). It sends the request
  with the output of the target workspace's group (the request needs one), falling back to an
  output the window is on. An unknown key or a missing capability drops the command, as F22.
  The ignored live test `the_amended_workspace_commands_take_effect_in_nested_cosmic` checks
  the published fields, pin/unpin and the move against nested cosmic-comp (result below, F82).
- **F79 `TrayCommand::Scroll`.** `system-tray` 0.8.9's `Client` has no scroll call and keeps its
  bus connection private, so the host opens a second session connection of its own at start
  and sends `org.kde.StatusNotifierItem.Scroll(delta, orientation)` to
  `<destination>/StatusNotifierItem` (the only path the host addresses, F29), with
  `orientation` `vertical` or `horizontal` and `delta` the notches unchanged. The sign
  convention (positive is down or right, `wl_pointer`'s) is what the bar will send; which way
  a given item treats a positive delta has not been checked against a real item yet. Without
  a bus the scroll is dropped, as the other commands' failures are logged and dropped.
- **F80 `sill debug click`, primary only.** The daemon maps `DebugRequest::Click` onto
  shell-host's `Inject::Click { at }`, which presses and releases the **primary** button and
  has no button field (shell-host `inject.rs`), so `--button secondary|middle` answers
  `Failed(Unsupported)` naming G10 rather than sending a primary click in its place (tested:
  `a_debug_click_maps_onto_shell_hosts_injection`). The dock's context menu (secondary press,
  design/10 §10.3.7) therefore cannot yet be exercised through `sill debug`. `dev/accept-bar.sh`'s
  menus step now clicks the daemon's own bar with `sill debug click` (the clock, 60 px from the
  right end) and dismisses with a click on the bar's empty middle; a popup's presence is read
  as whether `sill debug capture <bar>/popup` succeeds (`sill debug stats` lists bars only), so
  `accept-bar-ui.sh cosmic` and `bar_demo` are no longer part of the daemon's acceptance. G8
  is resolved by this. Run on the already-running nested cosmic-comp (`wayland-1`, 2176 x 1360,
  `sill run` from this branch with `--features debug`): all eight steps pass; the maximised
  probe is 1328 = 1360 - 32, the bar holds 5 frames and 5 commits across five idle seconds, and
  a click at (2116, 16) opened `sill-bar-WINIT-0/popup` (the clock menu, no measure panic now
  that the popup root carries `MEASURE`) and a click at (1088, 16) dismissed it.
  `dev/out/bar-cosmic-menu.png` is now the daemon's own bar with its clock menu open.
- **F81 One `HostMeasure` for every root.** Q10's twelve lines moved from
  `surfaces/bar/measure.rs` to `sill-surfaces/src/measure.rs` as `pub const MEASURE`. The bar
  and its menu popup no longer provide it themselves: `daemon::surface_root` puts `Services`,
  `SettingsFeed` and `MEASURE` on every surface root the daemon spawns, so the wallpaper, dock
  and launcher get it by construction. A popup is its own `VirtualDom` and inherits nothing
  from the surface that opened it, so the two popup roots the bar opens (the menu popup and the
  tray's dbusmenu popup, which had none) add `.context(crate::MEASURE)` themselves; so does
  `examples/bar_demo.rs`, which builds its own root. When quire exports ds-native's measurer
  (Q10) this module becomes a re-export.
- **F82 What the dock and the switcher need, checked against the amended vocabulary.** design/10
  §10.3.6-10.3.7 and design/13 §13.3.5 ask the compositor for: activate the most recent
  toplevel, restoring it when minimized (`Activate`, which unminimizes first, F22); cycle an
  app's toplevels (`Activate` of the next key); Hide and Option-click (`Minimize` per
  toplevel); Quit (`Close` per toplevel); a picked window of the context menu (`Activate`).
  All of these existed. Force Quit is a SIGKILL to the app's `app-sill-*` scope, which is the
  launch service's, not `WlCommand`'s. The minimize target (§10.3.10) is the one thing still
  missing, and deliberately (F76, G2). The dock's per-frame drive is not a sill signature:
  shell-host `df9c59b` added `use_frame_tick()` and `SurfaceHandle::drive_frames() ->
  FrameDrive`, which resolves G5, and shell-host F36 adds the rule the dock machine's settle
  must follow: its drop condition must be reachable at any tick rate (nested cosmic-comp calls
  back every 1-2 ms), so dock-ui's settle test needs a 1 ms case beside 16.7 ms. Live, on the
  running nested cosmic-comp 1.8 (`wayland-1`, one output `WINIT-0`) with `cosmic-term`:
  both workspaces published `coordinates` `[1]` and `[2]` and capabilities `[Activate, Rename,
  SetTiling, Pin, Move]`; `PinWorkspace` then `UnpinWorkspace` on workspace 1 each took effect
  in the published `pinning`; `MoveToplevelToWorkspace` moved the terminal from workspace 1 to
  2 (its `workspaces` became `[2]`). The existing live test's harness leaks `cosmic-term`
  processes that outlive the killed child (the terminal hands off to a second process), which
  also holds a piped test's stdout open; kill them after a run.
- **G5** RESOLVED (shell-host `df9c59b`, F36): `use_frame_tick` and `FrameDrive` (F82).
- **G8** RESOLVED by F80. **F23**, **F30** and **Q10**'s sill half are resolved by F77-F79
  and F81; Q10 itself (quire exporting ds-native's measurer) stays open.
- **G10** shell-host's `Inject::Click { at }` presses the primary button only; a
  `button` field (`MouseEventButton::Secondary`/`Auxiliary`) would let `sill debug click
  --button secondary` open the dock's context menu (F80).

## M3 dock-ui (2026-09-24)

Filled `surfaces/dock/**`: `Dock`, `Tile`, `dock_items`, `context_menu`, plus private modules
(`frame`, `hit`, `pointer`, `activate`, `effects`, `changes`, `pick`, `icon`, `menu_geometry`,
`menu_popup`, `session`, `view`) and one public addition, `dock::poke` (F97). The machine is
unchanged but for a new test file (`machine/settle_tests.rs`); no machine bug was found. Numbers
start at F92/Q15 so they cannot collide with m2-wallpaper's F83-F91 and Q14.

- **F92 Frames through shell-host's frame hook.** The session holds a `FrameDrive` from the
  machine's `RequestFrames` to its `StopFrames`, and a task awaiting `SurfaceHandle::next_frame`
  feeds `Frame` on every tick (not `use_frame_tick`, which panics without a `SurfaceHandle` and
  so could not render in an SSR test). `Redraw` and `StopFrames` both stamp the instant the
  render draws at, so the frame a leave reaches rest on is drawn too (the machine emits
  `StopFrames` alone there; design/10's table says `Redraw, StopFrames`; handled in the
  component rather than by changing the machine's tested output). `WakeAt` is a `ds::sleep`
  task feeding `Tick`. `machine/settle_tests.rs` drives enter, a tracked move, leave, an
  informational bounce and a released drag frame by frame at 1 ms and 16.7 ms ticks: each stops
  frames in bounded time (enter 120-160 ms, leave 200-220 ms, bounce 1000-1020 ms) at both rates.
- **F93 The root is `Material::Bar`, the pill a `Surface{Dock}`.** A `Ds` root of
  `Material::Dock` holding only positioned children is a zero-height, full-width box, and its
  drop shadow still painted a grey band down from the surface's top (seen in `sill debug
  capture`); Q13 again. The root takes Bar (no drop shadow) and the pill's box holds a nested
  `Surface{Dock}`, which paints the tint, edge, drop and 22 px radius. `dock.pill_radius_px`
  therefore does not reach the pill (a consumer may not style `.ds`); only the default 22 is
  drawn (Q15). The Space's hue is the bar's own tint layer (`--f-solid` at `--m-tint-alpha`,
  Q9, F48).
- **F94 Icons.** `IconView` of `IconSource::Image` (or `Symbolic` for a `*-symbolic` file) from
  the app's resolved `AppIcon::File` via `IconUrl::file`, `Glyph(Window)` when there is none
  (design/08 §4.3), `Glyph(Trash)` for Trash. The plate is 80.5 % of the tile (§2.2), neutral
  paper (`--raise` to `--surface-2`), `--shadow-2`, corner 22.37 % of its side; the icon is 72 %
  of the plate (§4.1), a glyph 56 %. `ds::IconSize` stops at 22 px (`Bar`), so the icon is drawn
  at 22 and scaled with `transform:scale` (Q16); on Blitz the themed SVGs stay sharp at 96 px
  (dev/out/dock-cosmic-magnified.png). §4.2's squircle detection (fill the plate when the icon
  already is one) is not built: COSMIC's own icons sit inside our plate.
- **F95 Items, ids and running state.** Running apps follow the pins in launch order, read as
  each group's smallest `WindowKey` (keys are handed out as toplevels are first seen). A tile's
  `TileId` is an FNV-1a hash of what it stands for (`app:<id>`, `raw:<app_id>`, `trash`), not
  its index, so a bounce stays on its app when another tile appears before it and a pinned and
  running app is one tile (the frozen `TileId` doc says "index-stable key"; the hash is stable
  across more). `TileApp::NoWindows` is never produced: nothing tells an alive, windowless
  D-Bus app apart from one still starting, and a second click then would launch it twice.
  `Active{windows}` counts the app's windows on every workspace; the cycle itself picks from
  the output's active workspace.
- **F96 Clicks, menu, drags.** Effects become commands in `effects.rs` (pure, table-tested):
  launch through `SurfaceHandle::activation_token` then `LaunchCommand::Launch` (the daemon's
  `launch.rs` is not reachable from a surface, so its two lines are repeated); middle click
  prefers the `new-window` action; `Cycle` activates the app's least recently used window on the
  active workspace, so repeated clicks visit every window (two alternate); Option-clicks
  minimize per §10.3.6; Cmd-click is `LaunchCommand::ShowInFiles`. The Trash tile reads as a
  running app to the hit test, so a click activates it (opening `trash:///` in COSMIC Files)
  instead of starting a ten-second launch bounce; its context menu is empty, so a secondary
  press on it opens nothing (Empty Trash needs a service sill does not have). The context menu
  (`Menu{Context}` in an `xdg_popup`, one per press, `Grab::Grabbed`) is flat for
  `WindowsFirst` (windows, desktop actions, Keep in Dock / Remove from Dock, Quit) and has the
  `Options` submenu for `AppleOrder`; the popup is placed above the tile as drawn (magnified),
  8 px gap, its height estimated (G7). Open at Login is not offered (no autostart writer);
  Option-held Force Quit is built but never shown: layer surfaces get no keyboard while the menu
  opens, so Option is always `Up`. The active window's `Check` is set but `Menu{Context}` draws
  no check column (its rows are `Tiled`), so it does not show. Minimized windows are prefixed
  `◆ ` (no ds diamond glyph). A drag reorders pins (`PinEvent::Move`), pins a running tile
  dropped before the last pin, and unpins past the pill grown by `dock.remove_threshold_px`;
  the ghost, the Remove label and the healing animation (§10.3.8) are not drawn (no ghost layer
  surface, no pointer grab outside the input region to test with).
- **F97 `dock::poke`, a harness's presses (G10).** shell-host injects primary clicks only, so
  nothing could press the secondary button on a dock under test. `DockPokes(watch::Receiver<
  Option<DockPoke>>)`, read with `try_consume_context` and so absent from the daemon's docks,
  lets `examples/dock_demo.rs` feed the same move-and-press a pointer would (stdin `sill-dock-X
  press secondary X Y`). It is a new public module; remove it once G10 is closed.
- **F98 The hover label is ds `Tooltip{Fly}`,** whose own `:hover` rule and `--d-fly` delay show
  it and HoverHub's warm state skip the delay. The machine's `ShowLabel`/`HideLabel` are
  therefore not applied: the Fly has no controlled mode, so the label does not hide on press or
  while a menu is open as §10.3.4 asks (Q17).
- **F99 Not built:** auto-hide is wired (strip element, slide offset, the machine's region
  effects) but not exercised live; overflow shrinking (§10.3.1), file drops and spring-loading
  (G4), minimize targets (G2), UI sounds (M6), the pressed scrim (the frozen `Tile` props carry
  no press), a launch bounce ending on `ScopeExited`/`LaunchFailed` (the 10 s cap ends it), the
  progress bar style (`ProgressStyle::Bar` shows nothing). The ring is a `conic-gradient`.
- **F100 Daemon.** `run_event` spawns a dock per output after the bar (`spawn_missing_docks`,
  the bar's pattern), and `collect_stats` reports `sill-dock-<output>` beside the bars, which the
  acceptance needs for its frame counts: two lines outside "the lines that spawn the dock",
  flagged here. The dock follows later settings changes to its own layer (size, exclusive zone,
  regions) through its `SurfaceHandle`. Side effect: `dev/accept-bar.sh` step 5 now sees a
  maximised window of `output_h - 32 - 72` and needs the dock's reserve subtracted (not edited:
  not owned here).
- **F101 Acceptance** (`dev/accept-dock.sh`, nested cosmic-comp 1.8 on the running `wayland-1`,
  2176 x 1360, `sill run` debug build with `--features debug` and a scratch `XDG_CONFIG_HOME`;
  `sill debug stats` counters are cumulative, so the script takes differences): (1) a maximised
  probe is 1256 = 1360 - 32 - 72; (2) 60 `sill debug pointer` moves in 1.000 s gave 51 frames for
  52 callbacks (0.98), the leave's shrink 16 frames, then 0 frames in the next 5 s; (3) a click
  on the pinned COSMIC Terminal tile launched it (`app-sill-com.system76.CosmicTerm-1.scope`)
  and its running dot was in the dock's capture 561 ms later; (4) `badge_emitter
  com.system76.CosmicTerm --count 5` turned the badge's pixel red in 377 ms; (5) `sill debug
  click --button secondary` answers G10, so the menu opened through `dock_demo` after the daemon
  quit (its popup `sill-dock-WINIT-0/popup`, 268 x 267, captured). Screenshots:
  `dev/out/dock-cosmic.png` (rest, with the probe's unmatched window as a running tile),
  `dev/out/dock-cosmic-magnified.png`, `dev/out/dock-cosmic-menu.png` (the demo's Terminal menu:
  two windows, New Window, Remove from Dock, Quit).

### Gaps found in M3 dock-ui

- **Q15** `Surface` (and a `Ds` root) take the material's radius with no override, so
  `dock.pill_radius_px` cannot reach the Dock pill (F93); and a zero-height root still paints its
  material's drop shadow (Q13, now seen for Dock as well as Popover).
- **Q16** `ds::IconSize` has no dock-tile size (48-128 px) and `IconView` no pixel size, so the
  dock scales a 22 px icon (F94).
- **Q17** `Tooltip{Fly}` is `:hover`-driven only; a controlled mode (shown/hidden by the caller)
  would let the dock machine's label rules (hide on press, while a menu is open, while
  dragging) apply (F98).
