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
`menu_popup`, `session`, `view`). The machine is
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
  8 px gap, its height estimated from the entries, wrapped titles included (shell-host's
  `FitContent` now measures a popup, but the menu is an overlay placed from the popup's bottom
  edge by the size given, so `Fixed` stays). Open at Login is not offered (no autostart writer);
  Option-held Force Quit is built but never shown: layer surfaces get no keyboard while the menu
  opens, so Option is always `Up`. The active window carries a check mark. Minimized windows are prefixed
  `◆ ` (no ds diamond glyph). A drag reorders pins (`PinEvent::Move`), pins a running tile
  dropped before the last pin, and unpins past the pill grown by `dock.remove_threshold_px`;
  the ghost, the Remove label and the healing animation (§10.3.8) are not drawn (no ghost layer
  surface, no pointer grab outside the input region to test with).
- **F97 Right-click through the real event path.** shell-host `51cdfa7` (G10, closed while
  this wave ran) injects a click with a button and sill master's daemon maps it, so `sill
  debug click --button secondary` reaches the dock as Blitz's `mousedown` with the secondary
  button, which the dock feeds the machine as a press (the menu opens on press, design/10
  §10.3.7). An interim harness seam for the demo was removed again before commit.
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
  flagged here. The branch merged master (the daemon's `to_host_button` mapping) before its
  gates. The dock follows later settings changes to its own layer (size, exclusive zone,
  regions) through its `SurfaceHandle`. Side effect: `dev/accept-bar.sh` step 5 now sees a
  maximised window of `output_h - 32 - 72` and needs the dock's reserve subtracted (not edited:
  not owned here).
- **F101 Acceptance** (`dev/accept-dock.sh`, nested cosmic-comp 1.8 on the running `wayland-1`,
  2176 x 1360, `sill run` debug build with `--features debug` and a scratch `XDG_CONFIG_HOME`;
  `sill debug stats` counters are cumulative, so the script takes differences), all five pass:
  (1) a maximised probe is 1256 = 1360 - 32 - 72; (2) 60 `sill debug pointer` moves in 1.000 s
  gave 52 frames for 52 callbacks, the leave's shrink 17 frames, then 0 frames in the next 5 s;
  (3) a click on the pinned COSMIC Terminal tile launched it (`app-sill-com.system76.CosmicTerm-1
  .scope`) and its running dot was in the dock's capture 502 ms later; (4) `badge_emitter
  com.system76.CosmicTerm --count 5` turned the badge's pixel red in 131 ms; (5) `sill debug
  click --button secondary` on that tile opened `sill-dock-WINIT-0/popup` (its window, Remove
  from Dock, Quit). Screenshots: `dev/out/dock-cosmic.png` (rest, the probe's unmatched window as
  a running tile), `dev/out/dock-cosmic-magnified.png` (pointer between the two tiles),
  `dev/out/dock-cosmic-menu.png`; the last shows Q17: the hover label stays under the open menu.
  `examples/dock_demo.rs` runs the dock alone on fake services (four tiles, a count of 7, a 40 %
  ring) for manual checks.

### Gaps found in M3 dock-ui

- **Q15** `Surface` (and a `Ds` root) take the material's radius with no override, so
  `dock.pill_radius_px` cannot reach the Dock pill (F93); and a zero-height root still paints its
  material's drop shadow (Q13, now seen for Dock as well as Popover).
- **Q16** `ds::IconSize` has no dock-tile size (48-128 px) and `IconView` no pixel size, so the
  dock scales a 22 px icon (F94).
- **Q17** `Tooltip{Fly}` is `:hover`-driven only; a controlled mode (shown/hidden by the caller)
  would let the dock machine's label rules (hide on press, while a menu is open, while
  dragging) apply (F98).

## M4 perf-ime (2026-09-24)

- **F102 `launcher.open_latency_ms` is timed at the IPC request's arrival, not inside
  `dispatch_launcher`.** `run_event`'s `HostEvent::App(ShellMsg::Ipc { .. })` arm starts an
  `Instant` for exactly `Request::Launcher(LauncherRequest::Toggle)`, calls
  `dispatch_request` unchanged, then logs the elapsed time — regardless of what the response
  turns out to be. This is the wave-ownership seam DESIGN.md's M4 table draws (perf-ime owns
  the latency lines, launcher-ui owns "spawn/route" in the same file): launcher-ui can rewrite
  `dispatch_launcher`'s body freely — spawn the two surfaces, wait for the shown frame, whatever
  the design needs — without touching this edit, and the number this line prints grows on its
  own from "IPC round trip only" (today, `dispatch_launcher` answers `Unsupported` at once) to
  "IPC arrival to the frame that presents the shown panel" the moment `dispatch_request` itself
  takes that long, with no further edit here. `open_latency_line`/`parse_open_latency_ms`
  (`crates/sill/src/daemon.rs`, both `pub` for `crates/sill/tests/latency.rs` and
  `dev/accept-launcher.sh`) are the pure formatting/parsing pair; `crates/sill/tests/latency.rs`
  checks both without a wall clock (fixed `Duration`s), including that the parser ignores every
  line that is not ours (an `Unsupported` failure message, another daemon line, a malformed
  number).
- **F103 `dev/accept-launcher.sh` runs three steps, self-contained** (own `sill run --features
  debug`, scratch `XDG_CONFIG_HOME`, on the already-running nested `wayland-1`; quits its daemon
  at the end via a `trap`), all with real evidence rather than assumed:
  1. **Real pass, today.** 20 `sill launcher toggle` calls (each answers `Failed(Unsupported)`
     and exits nonzero — expected, and irrelevant: F102's line is printed before the client ever
     sees the response) produced exactly 20 `launcher.open_latency_ms` lines on this machine:
     p50 0.000 ms, p95 0.001 ms, max 0.002 ms, comfortably under the 100 ms budget. This is
     IPC-only overhead, not the real open latency the plan's gate means — but the mechanism is
     real, and F102's design means it starts measuring the real thing unchanged once M4
     launcher-ui lands, with nothing in this script to update.
  2. **Skip (exit 2), "M4 launcher-ui."** `sill debug stats` carries no `sill-launcher-*` row
     after a toggle (`dispatch_launcher` never spawns anything yet), so the panel-exists and
     5 s-idle-after-close checks cannot run; the step checks for the row itself rather than
     assuming its absence, so it starts passing for real the moment a launcher surface exists.
  3. **Skip (exit 2), reason logged but not diagnosed further.** The nested `custom` shortcut
     (`dist/cosmic/.../custom`, mirrored into shell-host's `scripts/dev/xdg-config`, already
     loaded when this machine's `cosmic-comp` started) is `(modifiers: [Ctrl, Alt], key:
     "space"): Spawn("sill launcher toggle")`. `wtype` is installed here (`ydotool` is not); the
     step sends `wtype -M ctrl -M alt -k space -m alt -m ctrl` through the nested compositor and
     watches the daemon's log for a new `launcher.open_latency_ms` line for 5 s. Live result: no
     new line. `comp.log` names neither a shortcut match nor a spawn attempt at any log level
     enabled by default, so the cause is not distinguished between two candidates: `cosmic-comp`
     resolves `Spawn`'s bare `sill` on its own inherited `PATH`, fixed when `dev/nested.sh`
     started the compositor (before any `cargo build` in this session put a `sill` binary
     anywhere); or a `wtype`-injected key, which travels the `virtual-keyboard-unstable-v1`
     protocol rather than a real input device, never reaches `cosmic-comp`'s global-shortcut
     matcher at all. Either is a shell-host/cosmic-comp fact, not something `dev/`-owned code can
     fix; recorded rather than guessed at further.
- **F104 `dev/ime-probe.sh` proves the two checks it can and stops.** Live on this machine,
  right now: `loginctl`'s seat0 session (found by its `SEAT` column, not `$XDG_SESSION_ID` — a
  script invoked from a pts session must not check that session's lock state) has
  `LockedHint=yes`, so the script exits 2 with that fact named, exactly as running it demonstrated
  (`./dev/ime-probe.sh` printed the lock message and exited 2 without touching fcitx5 or the
  compositor). fcitx5 answers on the session bus (`busctl --user call org.fcitx.Fcitx5
  /controller org.fcitx.Fcitx.Controller1 State`, checked separately, unlocked or not — D-Bus
  calls do not need keyboard focus). Past those two checks the script looks for
  `sill-surfaces/examples/launcher_demo`, then `bar_demo`; neither offers a real text-input-v3
  field yet (`bar_demo` has none at all — the bar has no text entry), so it always stops there,
  exit 2, and prints the exact commands for the rest. **Correction to the brief's D-Bus name**:
  this machine's fcitx5 does not answer on `org.fcitx.Fcitx5.Controller1` — introspecting
  `org.fcitx.Fcitx5` `/controller` shows the interface is `org.fcitx.Fcitx.Controller1` (no
  "5"), which is what the script and its printed commands use. **`SetCurrentIM`'s effect could
  not be confirmed live while writing this**: calling `SetCurrentIM s chewing` then
  `CurrentInputMethod` returned `""` before and after (session locked throughout, so no
  wayland client had an enabled, focused `zwp_text_input_v3` context for fcitx5 to apply a
  "current" method to) — the printed commands document the call, not a verified round trip;
  that verification is exactly what running them against a real field, once one exists, is for.
  Facts recorded for the documented commands rather than assumed: `wtype` is installed on this
  machine, `ydotool` is not; the Default input-method group here is `keyboard-us` + `chewing`
  (Taiwanese Zhuyin/Chewing — the only CJK method configured), matching shell-host FINDINGS.md's
  own note that `fcitx5` is installed and IME has only been exercised on KWin so far (`text-
  input-v3` accumulates `preedit_string`/`commit_string`/`delete_surrounding_text` until `done`,
  `crates/shell-host/src/ime.rs`).
## M4 providers-a (2026-09-24)

Filled `providers::{apps, windows, system}` (`sill-services/src/providers/{apps,windows,system}.rs`);
`sill-launcher/**`, `results/**`, `providers/{calculator,settings}.rs` and every other M4 file
stayed untouched. `sill-services/Cargo.toml` gained one line, `nucleo-matcher = { workspace =
true }` — already pinned (root `Cargo.toml`, used by `sill-launcher`), just not yet wired into
this crate, which needs the concrete `nucleo_matcher::Matcher` type in provider function
signatures that `sill_launcher::matching::matcher()` returns.

- **F105 `Activation` cannot close a window; the row still offers the action.** The brief asks
  the windows provider to close with an action, but the frozen `sill_launcher::Activation`
  (owned by launcher-core) is `Launch | FocusWindow | Copy | OpenSettings | System | Nothing` —
  no variant carries a `WindowKey` to a close request, and `WlCommand::Close` (the compositor
  command that exists for it, `cosmic_wl::command.rs`) has no path from `Provider::activate`,
  which is documented pure ("it says what should happen and the daemon does it") and so cannot
  call it directly either. `WindowsProvider` still gives every row a `Close` action (so the row
  shape is right and the gap is visible in the actions menu, Ctrl+K / Alt+K) but
  `activate(item, Choice::Action(_))` can only answer `Activation::Nothing`. Recommend
  launcher-core add `Activation::CloseWindow(WindowKey)` (mirroring `FocusWindow`'s shape) in a
  follow-up; this is a stop-and-report, not a workaround (CONVENTIONS §1).
- **F106 The "confirmation flag" is the actions menu, not a new field.** design/06 §20.2 gives
  the launcher exactly one lever in the frozen `Item`/`Choice` shape for anything beyond Enter:
  Ctrl+K / Alt+K opens `Item::actions`. `Item` has no boolean (and CONVENTIONS §11 forbids one
  in a public signature regardless) and `Activation` has no "ask first" wrapper, so
  `SystemProvider` reads Log Out, Restart and Shut Down as destructive (they close or interrupt
  every running app) and gives each one action, `Action{id: "confirm", label: "Confirm
  <title>"}`; `Choice::Primary` on one of these rows answers `Activation::Nothing`, and only
  `Choice::Action("confirm")` answers the real `Activation::System(command)`. Suspend is treated
  as non-destructive (instantaneous, reversible, no app is closed) matching macOS and GNOME;
  Lock and Reload need no confirmation at all. This is a considered reading of an unspecified
  design point, not a verified one — worth a second look once launcher-ui's actions menu exists
  to see how it reads in practice.
- **F107 Session-command availability is a live, cached logind probe, not a static list.**
  `SystemProvider::query` lists Suspend, Restart and Shut Down only when logind's own read-only
  `Manager.CanSuspend`/`CanReboot`/`CanPowerOff` (`Yes` or `Challenge`) says so, and lists
  nothing extra when the bus or logind is unreachable at all (`Offer::Unavailable`, the wave's
  "reports `Unavailable` cleanly" requirement) — Lock, Log Out and Reload need no such check and
  always list. The check is real D-Bus (async `logind_zbus::manager::ManagerProxy`, not the
  `Blocking` variant: `zbus`'s pinned features here are `default-features = false, features =
  ["tokio"]`, so `blocking-api` — and with it `zbus::blocking` and every `*ProxyBlocking` type —
  is not compiled in; adding it would be a pinned-dependency change, out of scope for a single
  crate per CONVENTIONS §11) bridged from `Provider::query`'s sync signature with
  `tokio::runtime::Handle::block_on`, which is sound exactly because `results::spawn` (M4
  launcher-core, plan "spawn_blocking per provider") is documented to run every provider's
  `query` inside `tokio::task::spawn_blocking` — the one context `Handle::block_on` does not
  panic from. The probe runs once and is cached in a `OnceLock` for the provider's lifetime (a
  fresh `Connection::system` per keystroke would risk the 50 ms budget for no benefit: session
  policy does not change keystroke to keystroke). `tests/providers_system_live.rs` (`#[ignore =
  "needs logind"]`) exercises the real probe through the same `spawn_blocking` seam and only
  ever calls the read-only `Can*` properties — never `Suspend`/`Reboot`/`PowerOff`/
  `TerminateSession`, so running it does not touch the session it runs in. Run once on this
  machine: `Lock, Log Out, Reload, Restart, Shut Down` (Suspend absent — `CanSuspend` answered
  `No`, most likely because this dev box has no swap/hibernation image configured for logind to
  offer it).
- **F108 Desktop actions surface as their own rows only once something is typed.** The brief
  asks for "desktop actions as secondary items"; `AppsProvider` matches a `[Desktop Action]`'s
  own name as a separate `Item` (its `Activation::Launch` carries `DesktopAction::Named`, its
  id `<app id>\u{1}<action id>` — `\u{1}` rather than a printable separator such as `::`,
  since CONVENTIONS "substrings are not tokens" argues for a joiner no real id can contain
  rather than one that merely seems unlikely to collide) alongside its owning app's own row,
  which still lists every desktop action in its own `actions` menu regardless of whether the
  action text itself matched. An empty query skips the per-action rows (every app already lists
  by frecency alone; also listing every one of their actions would be a wall of rows with
  nothing typed) — a considered reading, not specified by the brief.
- **Icon glyphs.** design/08-ICONS.md §1.6 catalogues only `lock` and `power` among session
  glyphs, and `window` for "windows provider, dock menu" (confirming the windows provider's own
  fallback icon by name). `SystemProvider` gives Lock its own `lock` glyph, every other
  power-menu row `power`, and Reload `IconRef::None` (not a power-menu action, and no glyph for
  it is catalogued — `None` falls back to the provider's own glyph per `IconRef::None`'s own
  doc, rather than inventing an uncatalogued name locally, per CONVENTIONS §11 "a missing …
  token is added to quire first").
- **`WindowsProvider` gained a third field,** `workspaces: watch::Receiver<Workspaces>` (the
  stub had only `toplevels` and `apps`): the brief's row shape is "title, app name, workspace",
  and a `Toplevel` only carries `WorkspaceKey`s, not names — `cosmic_wl::spawn` already
  publishes `Workspaces` (workspace names) beside `Toplevels`, so the third receiver is simply
  wired in rather than left unresolved.

### Tests

`cargo test -p sill-services --lib providers::` — 19 unit tests (matching/ranking tables,
cancellation, `activate` mappings, id encode/decode), all pass. `tests/providers_apps.rs` runs
`AppsProvider` against the real fixture tree `tests/fixtures/applications` (the same one
`tests/scan.rs` scans): name/generic-name/keyword matches, a matching desktop action listing as
its own row, and `Hidden`/`NoDisplay`/an unresolvable `TryExec` never listing. `tests/
providers_windows.rs` runs `WindowsProvider` against a fake `Toplevels`/`Workspaces` pair (no
compositor). `tests/providers_system_live.rs::lists_session_commands_against_real_logind`
(`#[ignore = "needs logind"]`) is the live test described in F107; run once, reported there.
## M4 providers-b (2026-09-24)

Filled `providers::calculator` and `providers::settings` (`CalculatorProvider`,
`SettingsProvider`), each split into a small directory under `providers/{calculator,settings}/`
per CONVENTIONS §8/§11 (one concept per file).

- **F109 `sill-services` cannot depend on `ds-settings`, so the settings provider re-reads
  `*.settings.toml` itself.** The brief asks the settings provider to index "the installed quire
  and sill schemas" through the schema types design/22 §9 defines (`ds_settings::schema::{Schema,
  KeySpec, discover, deep_link, ...}`), but `ds-settings` depends on `dioxus` unconditionally
  (quire FINDINGS F2) and `scripts/check-boundary.sh` mechanically forbids `sill-services` from
  reaching `ds-settings` at all (the "UI-free" rule, DESIGN.md "Crates"). Rather than weaken the
  boundary or the brief, `providers/settings/schema_file.rs` reads the same on-disk TOML shape
  directly with `toml`/`serde` (only the fields the launcher's search needs: `path`, `label`,
  `help`, `page`), and `providers/settings/index.rs::SettingsIndex::deep_link` reimplements
  `ds_settings::schema::deep_link`'s exact algorithm (unique case-insensitive label/help
  substring match) independently rather than calling it. Stop-and-report per CONVENTIONS §1;
  `data_dirs()` is likewise a second, independent reading of the same XDG directories
  `ds_settings::schema::data_dirs` resolves (the general shape of this duplication is F59/Q4's,
  not new). `tests/settings_live.rs` needed `sill_settings::schema()` for one live test only
  (design/22 §9.2's own local-install path); that is a `[dev-dependencies]` entry in
  `sill-services/Cargo.toml`, which `check-boundary.sh`'s `-e normal,build` edge filter does not
  see (confirmed: `cargo tree -p sill-services -i ds-settings -e normal,build` prints nothing,
  and `./scripts/check-boundary.sh` passes) — it is never linked into the daemon.
- **F110 The calculator's secondary "Insert" action does not exist.** The plan and this wave's
  brief both ask for "a secondary 'Insert' action if the frozen vocabulary has it"; it does not.
  `sill_launcher::Activation`'s only text-carrying variant is `Copy { text }` — there is no
  "put this back in the search field" kind. `CalculatorProvider::query` yields the `Copy` row
  only, with an empty `actions` list. Reported rather than worked around by adding a frozen
  variant (that is a cross-crate, coordinated change, not this wave's to make).
- **F111 "bluetooth" is design/22's own example of a query that does *not* deep-link to a file
  key.** §9.3 gives two deep-link examples side by side: "dark mode" resolves to
  `appearance.theme` through the schemas, and "bluetooth" resolves through a *live* D-Bus
  settings module (§9.4, `org.quire.SettingsModule1`) — nothing in quire or sill implements a
  live module yet. `SettingsIndex::deep_link("bluetooth")` correctly returns `None` against both
  the fixture schemas and the real ones (`tests/settings_fixture.rs`), which is right, not a
  regression: a live-module deep link is out of this wave's scope, and inventing a static key
  match for it would be worse than reporting the gap.
- **F112 The 50 ms budget is enforced cooperatively, not by killing a thread.** fend-core's
  `Interrupt::should_interrupt` is polled between steps of its own evaluation (each factorial
  multiplication, for instance) — there is no way to hand it a signal that preempts mid-step, and
  matching its returned error string to tell "interrupted" from "malformed input" would be
  exactly the substring-matching CONVENTIONS warns against (its error text is not a vocabulary
  this workspace owns). `providers/calculator/eval.rs::evaluate` instead re-reads the clock after
  a failed evaluation: an error at or past the budget is `TimedOut`, one before it is `NoMatch`.
  `a_pathological_expression_is_abandoned_within_the_budget` (`1000000!`) exercises this for real
  and finishes in well under 4x the budget.
- **F113 Live schema counts (this machine, 2026-09-24).** `tests/settings_live.rs` (writes
  sill's schema into the real `$XDG_DATA_HOME/quire/settings`, cleans up the one file
  afterward): sill's own schema has **142 keys** (its 11 domains merged, `sill_settings::schema`);
  `cosmic-settings` 1.7.0 is on `PATH`, so **33 COSMIC pages** load into
  `SettingsProvider::cosmic_pages`; **0 quire keys** are installed on this machine — quire ships
  no program that aggregates and writes its own `quire.settings.toml` yet (only the per-struct
  `AppearanceSettings::schema()`/`IconsSettings::schema()` derive output, each its own `Schema`
  value with nobody calling `.write_to()` on it), unlike sill's `sill_settings::schema()`
  (`crates/sill-settings/src/schema.rs`) which already merges its 11 domains into one. Not this
  wave's gap to fix (quire's), but worth flagging upstream: a quire binary calling
  `--write-schema` the way `sill`'s `main.rs` does would need writing.

## M4 launcher-core (2026-09-24)

Filled `results::spawn` (the runner), the `frecency.json` I/O and the `services.rs` placeholder
line (`results/{mod,run,sink,arrange,history,registry,effect,frecency_file}.rs`); amended
`sill-launcher::Activation` (F116). Latency: `examples/search_bench.rs` and the
`results::tests::latency` test (F118).

- **F114 Groups are an ordering of the flat `Results::items`, read back by rule.** The frozen
  `Results { generation, items }` has no section labels, so design/06 §11.3's groups are the
  published order (`results/arrange.rs`), and the launcher list derives its `SectionHeader`s
  from it. Typed text: row 0 is **Top hit** (the single best row of every provider, not
  repeated below); then one run per provider in Tab order (`ProviderKind::ALL`), best first,
  capped Apps 6, Windows 3, Calculator 1, Settings 3, System 3 (§11.3 caps Mail 6 and People
  and Actions 3; apps take Mail's). A header goes before row 1 and wherever the provider changes.
  Nothing typed (whitespace counts as nothing, `Needle::is_empty`): **Actions** = every System
  row, then **Recent** = the 5 most frecent non-System rows (never-chosen rows are not shown);
  the list knows the query it sent, so "empty query: System prefix is Actions, the rest Recent".
  The caps and the Recent count are proposed numbers with no settings key (design/22 has none;
  a key is a design/22 change first). Side effect: Tab's `Filter::Only(provider)` can show at
  most cap (+1 top hit) rows of that provider; an uncapped per-provider view needs a frozen-shape
  change (a group field or an uncapped list), reported rather than worked around.
- **F115 Frecency is one shared history; only apps rank with it.** The runner's `History`
  (`Arc<Mutex<Frecency>>`) is the same `Arc` `AppsProvider::frecency` holds (`registry.rs`),
  so a choice counts from the next keystroke; each change is saved by a single saver task
  (newest version wins, temp file and rename, lenient load). `spawn(rt, providers)` keeps its
  frozen signature: providers built by its caller rank with their own handle, so the daemon
  goes through the crate-private `spawn_with_providers(rt, Wiring{..})`. Items carry only
  their provider's final `Score`, so the runner cannot add frecency to rows whose provider
  has no handle (windows, settings, system, calculator): those rank by match alone under typed
  text, and every provider's rows reach Recent. The runner does not re-rank; `rank()` runs in
  the providers. The runner publishes a generation's first rows when its first non-empty batch
  arrives (keeping the previous rows up until then, so typing does not flash an empty list),
  and an empty list once every provider has finished with nothing; a provider that panics
  still counts as finished (a drop guard), and a panic in `activate` is caught.
- **F116 Freeze amendment: `Activation::CloseWindow { window: WindowKey }` and
  `Activation::Insert { text: String }`** (coordinator request, closing F105 and the
  calculator's secondary action). Struct variants like `FocusWindow { window }` and
  `Copy { text }`, adjacently tagged: `{"kind":"close_window","v":{"window":7}}`,
  `{"kind":"insert","v":{"text":"42"}}` (every variant's wire form is pinned in
  `activation.rs`'s test). `WindowsProvider::activate` (providers-a's file) still answers
  `Nothing` for Close until its owner switches it to `CloseWindow`; the calculator's `Insert`
  is providers-b's.
- **F117 The runner carries out window and app activations.** `SearchCommand::Chosen{key,
  choice}` finds the row among the newest generation's rows, asks its provider's `activate`,
  and routes the answer (`results/effect.rs`): `FocusWindow` -> `WlCommand::Activate`,
  `CloseWindow` -> `WlCommand::Close`, `Launch` -> `LaunchCommand::Launch` (no activation
  token: the surface has none to hand over yet). `Copy`, `Insert`, `OpenSettings`, `System`
  and `Nothing` are not the runner's (clipboard, the query field, settings, logind): the
  launcher surface or the daemon must act on them, and nothing does yet. Because the lookup
  uses the current rows, the surface must send `Chosen` before `Cancel` (closing clears them).
- **F118 Latency, query to first published batch** (through `Control::send`, a new
  generation, a blocking task per provider, the channel, `reduce_results`, grouping and the
  watch; a stand-in apps provider doing the real work, `rank_text` over name, generic name and
  keywords, since the real one landed in parallel). Test build, fixture index (11 listed
  apps), 200 queries after warm-up: p50 24 us, p95 32 us, max 60 us (asserted p95 < 10 ms).
  Release (`cargo run -p sill-services --example search_bench --release`): fixture p50 8 us,
  p95 11 us; 1500 apps p50 88 us, p95 183 us, max 240 us.
## Adoption of the bar gaps (2026-09-24)

quire master merged `tray-gaps` (Q6-Q8) and `bar-gaps` (Q9-Q13, the status items); this pass
removes the bar's, the dock's and the tray's workarounds for them. Numbers start at F120/Q20 so
they stay clear of the M4 launcher branches running beside it.

- **F120 The measurer is quire's.** `sill-surfaces/src/measure.rs` is gone. The workspace names
  `ds-native` (a path dependency beside `ds`, outside the pinned block; it brings `winit` and
  `dioxus-native` into the lockfile, all licence-clean under `cargo deny`), and
  `sill_surfaces::MEASURE` is now a re-export of `ds_native::measure::MEASURE`, so
  `daemon::surface_root` (and the launcher's code, which may name it) is unchanged in shape: a
  `Root` is built outside any component, so `.context(MEASURE)` is how it provides what
  `provide()` provides inside one. The three popup roots (the bar's menu popup, the tray's
  dbusmenu popup, the dock's context menu) call `ds_native::measure::provide()` at their top
  instead of taking a context from their opener. `sill-surfaces` no longer needs
  `dioxus-native-dom` directly.
- **F121 The bar draws the Space through its root.** The `sill-bar-tint` layer is deleted; the
  `Ds` root (already given the workspace's `SpaceLook`) takes quire's defaults for a Bar:
  `FrameTint::Tinted` and `Ground::Frame`. Live, the bar is the Space's gradient at the Bar's
  alpha with its grain (`dev/out/bar-cosmic.png`). The status items are
  `IconButton{Status}` in a group whose inline style carries `ds::StatusMetrics`, filled by
  `status_item::status_metrics` from `bar.status_icon_box_px`, `bar.status_glyph_px` and
  `bar.glyph_size_policy` (`IconSizeBar22`: the glyph is the box), so Q12 and F49's first
  half are closed. An icon item's slot no longer draws the open-title pill (the button is
  `--f-pill` while `expanded`); the clock and the app name, which are text, still do
  (`title::SlotPill`). Wired networks show `Icon::Ethernet`. The status lines of the network,
  battery and clock menus are `MenuEntry::Info` (F47 closed), sized as item rows in the popup's
  estimate.
- **F122 The menu machine hears the menu.** Bar menus are `MenuEntrance::Instant` (F50's
  entrance closed). The popup reports `on_hover` as `Hovered(choice)` and `on_release` as
  `Released(Option<Choice>)` (`plan::choice` maps a quire choice index to the enabled item's
  pick, numbered as quire numbers them), and `plan::popup_input` feeds them to the tracker as
  `Move(Target::Item | Menu)` and `Release(..)`. Because quire calls `on_release` before its own
  pick, the machine decides press-drag-release and click mode alike: a held release over an
  item picks and runs once, a held release over a status line or padding closes, a release in
  click mode over a status line keeps the menu; quire's `onpick` that follows finds the session
  closed and is ignored (table-tested in `plan.rs`). The keyboard's Enter still arrives as
  `Picked`. Whether the popup ever gets that release during a press-drag depends on the
  compositor handing the implicit grab over (F46); `sill debug click` cannot press on one
  surface and release on another, so it is proved by the table, not live.
- **F123 Popup roots are Popover again, and keep their room.** The bar's, the tray's and the
  dock's popup roots are `Material::Popover` (transparent chrome); the menu cards paint the
  material. The shadow room frame stays in both `bar_popup.css` and `dock_popup.css`: the card's
  drop (`--shadow-pop`, reaching 24 px beside and 40 below it) is painted by the card now, but
  the popup surface still clips at its own edges, so the room must be inside it. Live, the
  popup is transparent around its card (`dev/out/bar-cosmic-menu.png`,
  `dev/out/dock-cosmic-menu.png`).
- **F124 Bar popups are `PopupSize::Fixed`.** With the room frame and a transparent root, the
  bar's popup document has nothing in flow, and shell-host's content fit (its G7 fix, which
  `menu_popup`'s `FitContent` now reaches) measured it and shrank the popup to 1 x 1: the first
  acceptance run passed its "popup is up" check with an empty popup (a 268 x 129 capture, alpha
  0 everywhere, then 1 x 1). The bar's menus and the tray's dbusmenu popup now pass `Fixed` with
  the geometry's estimate, as the dock's context menu already did (F96). The tray popup's height
  is estimated from its entries (`geometry::menu_size`, capped at 640 px) instead of a fixed
  240; a long dbusmenu scrolls inside quire's menu.
- **F125 The dock's pill is its root.** The root is `Material::Dock` in a box at the pill's
  rectangle (`#dock-pill`, still the blur region's element), sized by an in-flow body of the
  pill's height; the tiles, dots, separator and hit elements hang from the root in a layer
  moved back to the surface's origin, so the frame's surface coordinates are unchanged. The
  nested `Surface{Dock}` and the `sill-dock-tint` layer are deleted; the pill is the Space's
  gradient at the Dock's alpha, with the material's edge, drop and 22 px radius
  (`dev/out/dock-cosmic.png`). The root passes `ground: Some(Ground::Paper)`: under the Dock's
  default frame ground the plates' `--raise`/`--surface-2` would turn into `--f-pill`, and
  design/08 §4.1 asks for neutral paper; the dot and the separator already name `--f-ink` and
  `--f-line`. `dock.pill_radius_px` still does not reach the pill: `Ds` has no radius override
  and `--m-radius` is declared on the `.ds` itself (Q15 stays open).
- **F126 The dock label follows the machine's gate, as far as the Fly allows.** quire's
  `Tooltip` still has no controlled mode (Q17 open), so the label stays a `Tooltip{Fly}` whose
  `:hover` shows it, now on a box over the plate beside it rather than wrapping it, and the
  tile leaves it out of the document while `machine::label_gate` is `Quiet`: a button down or a
  drag (the machine's `press != Up`), or a context menu open. The dock provides the gate as a
  `Signal<LabelGate>` context; the frozen `Tile` props are unchanged. Live, the label no longer
  shows under an open context menu (`dev/out/dock-cosmic-menu.png`, compare F101's Q17 note).
  The Fly's delay and warm skip remain quire's, not the machine's `ShowLabel` timing.
- **F127 Tile icons are still drawn at 22 px.** `ds::IconSize::Bar` (22) is still quire's
  largest and `IconView` has no pixel size, so the dock keeps scaling a 22 px icon (Q16 open).
- **F128 The tray draws each item's own icon.** `sill_services::tray::icon_url` and its
  hand-rolled base64 are replaced by `icon_art`: a pixmap's PNG bytes, a theme PNG's bytes, a
  theme file named `*-symbolic` (design/08 §1.5 step 1, compared at the last `-`), or another
  theme file. The bar's `tray_input::icon_source` makes the `IconSource`: a PNG by
  `ds::icon::classify_with` (step 2) at `icons.symbolic_chroma_max`, which quire master now
  registers in its own `IconsSettings` (quire's appearance file, thousandths, default 40): the
  tray reads it from the appearance feed the bar already carries
  (`tray_input::chroma_limit`, quire's default when out of the key's 0..0.2 range), so no
  sill-settings key is added; a symbolic file as `Symbolic`, anything else as `Image`, the
  window glyph when there is no art. The result is kept per item until its icon or the limit
  changes (theme lookup and a file read are not done
  per render). Items are `IconButton{Status}` with the `tray-<n>` id on the button itself (the
  wrapper span only listens for the wheel). Live, the coloured icons (Toshy's, the video
  bridge's) draw as images and the other two items in the frame ink (`dev/out/bar-cosmic.png`).
- **F129 Tray presses carry their point, and the wheel scrolls.** `tray_input::press_action`:
  primary opens the dbusmenu (or sends `Activate` without one), secondary opens the dbusmenu,
  middle sends `SecondaryActivate`; both activations carry `press.at` plus the output's logical
  origin (the bar sits at the output's top-left). A secondary press on an item without a
  dbusmenu would be SNI `ContextMenu(x, y)`, which the frozen `TrayCommand` does not have, so it
  does nothing. The bar does receive wheel events (shell-host sends `wl_pointer.axis` to Blitz
  as `wheel`, as lines with the sign negated), so a `wheel` over an item sends
  `TrayCommand::Scroll` with `tray_input::scroll_delta`: whole notches on the main axis turned
  back to `wl_pointer`'s sign, a half notch rounding to one, touchpad pixels and pages sending
  nothing. Not exercised live: `sill debug` has no wheel injection. Disabled dbusmenu rows are
  drawn disabled now; submenus are still dropped (the popup is not sized for one beside it).
- **F130 Acceptance** (built against quire master `175bfb9`, with the retuned tint alphas: Bar
  dark .68, Dock light .59 and dark .68), nested cosmic-comp 1.8 on the running `wayland-1` (2176 x 1360),
  `sill run` debug build with `--features debug`. `dev/accept-bar.sh`: all eight steps pass;
  the maximised probe is 1256 = 1360 - 32 - 72, the bar holds 5 frames and 5 commits across
  five idle seconds, the clock click opened `sill-bar-WINIT-0/popup` and the middle click
  dismissed it (the step's own `popup.png` is the popup's first frame, still empty; a capture
  1.5 s after the click is 268 x 129, alpha 0 at the corners and the card opaque in the
  middle). `dev/accept-dock.sh`: all five pass; maximised 1256; a 60-move sweep drew 46
  frames for 46 callbacks, the leave's shrink 12 frames, then 0 frames in 5 s; the terminal
  tile's click showed its running dot after 590 ms; the badge turned red in 405 ms; the
  secondary click opened `sill-dock-WINIT-0/popup`. By hand on the same daemon: a hover from
  the open clock menu onto the network item switched the one popup to the network menu (two
  status lines, then Settings), and the Toshy item's dbusmenu opened with its disabled status
  rows faint. Screenshots regenerated: `dev/out/bar-cosmic.png`, `bar-cosmic-menu.png`,
  `dock-cosmic.png`, `dock-cosmic-magnified.png`, `dock-cosmic-menu.png`.

### Gaps left after the adoption

- **Q15** (open) `dock.pill_radius_px` cannot reach the Dock root's radius: no override prop,
  and `--m-radius` is set on `.ds` itself (F125).
- **Q16** (open) no icon size above 22 px (F127).
- **Q17** (open) `Tooltip{Fly}` has no controlled mode; the dock only removes it (F126).
- **Q20** The tray's dbusmenu popup is not part of the bar's menu session: a click on the bar
  outside it does not close it, and a bar menu opened while it is up opens a second popup
  beside it (seen live). It predates this pass (F33); folding it into `menu_track` as a sixth
  kind of menu is the fix.
- **Q21** A dbusmenu submenu is still dropped: the tray popup would need room beside the menu
  for quire's submenu (quire "Tray gaps", "left for sill").
- **Q22** SNI `ContextMenu(x, y)` has no `TrayCommand`, so a secondary press on an item with no
  dbusmenu does nothing (F129).
- **G11** `sill debug` cannot inject a wheel event or a press-drag across surfaces, so the tray
  scroll and the bar's press-drag-release are proved by tables only (F122, F129).

## M4 providers follow-ups (2026-09-24)

Closed the three gaps F105, F110 and part of F109/F113 left open, now that F116's freeze
amendment (`Activation::CloseWindow { window }`, `Activation::Insert { text }`) is merged.

- **F105 closed: `WindowsProvider::activate` answers `CloseWindow`.** `Choice::Action(_)` on a
  window row (there is only the one, `close`) now answers `Activation::CloseWindow { window:
  window_key_of(item) }` instead of `Nothing`; `results/effect.rs` (M4 launcher-core, F117)
  already routes it to `WlCommand::Close`, so this was the last hop. The unit test renamed from
  "...is an acknowledged gap" to "...closes on the close action" and asserts the real variant.
- **F110 closed: the calculator's result row carries a secondary "Insert" action.**
  `result_item` now gives the row one `Action{id: "insert", label: "Insert"}` beside the
  implicit primary; `activate` matches on the chosen `ActionId` (`"insert"` -> `Activation::
  Insert{text}`, `Choice::Primary` or any other action -> `Activation::Copy{text}`, both
  built from the same id-decoded value so a future third action cannot desync the two). No unit
  tests existed for `CalculatorProvider::query`/`activate` before this (F110's report only
  covered `eval.rs`); added a small `#[cfg(test)] mod tests` in `calculator/mod.rs` proving the
  row carries exactly the one action and that both choices answer the activation F116 pinned.
- **F119 `cosmic_settings_available`'s `$PATH` search is now a pure, injectable function, so a
  test can prove COSMIC-page discovery without mutating the real environment.** The provider
  already discovered pages "when `cosmic-settings` is on `PATH`" (`SettingsProvider::new` ->
  `with_dirs` -> `cosmic_settings_available()`), but nothing tested that path, and it could not
  be tested as written: `cosmic_pages.rs::on_path` read the real `std::env::var_os("PATH")`
  directly, and `std::env::set_var` is `unsafe` since edition 2024 — forbidden everywhere but
  shell-host's one allowed module (CONVENTIONS §11) — so a test cannot point it at a fake
  directory by mutating `$PATH`. Split the search into `cosmic_pages::is_on_path(dirs, program)`
  (pure: does `program` name a file directly under one of `dirs`) and
  `cosmic_pages::real_path_dirs()` (reads the real `$PATH`, once); `SettingsProvider` gained a
  crate-private `with_dirs_and_search_path(dirs, search_path)` between `with_dirs` and the
  struct literal, so `with_dirs` still searches the real `$PATH` but a unit test can pass a
  fixture directory instead. `providers/settings/mod.rs`'s new `tests` module creates a throwaway
  directory holding one file named `cosmic-settings` (the same `is_file()` check `apps/scan.rs`'s
  own `on_path` uses for `TryExec`, not a permission-bit check) and proves `cosmic_pages` is
  non-empty when that directory is searched and empty when it is not. `results/registry.rs` now
  builds the provider with `SettingsProvider::new()` directly rather than through `Default`
  (`Default` already forwarded to `new()`, so this is a clarity change, not a behavior one).

## M4 launcher-ui (2026-09-24)

Filled `surfaces/launcher/**`: `Launcher`, `LauncherCatcher`, `ResultsList`, `ActionsList`,
`keys::step`, plus `presence`, `keymap`, `groups`, `entries`, `activate`, `session`,
`mounting` (public, pure, table-tested) and the private `panel`, `events`, `follow`,
`catcher`, `run`, `script`. Numbers start at F140/Q40/G20 so the parallel M4 waves cannot
collide.

- **F140 One shared launcher state; each surface maps itself.** `presence::LauncherLink` is a
  `watch::Sender<LauncherShown>` (`Hidden { last }` or `Shown(Opening { output, query, serial,
  requested, focus })`), created by the daemon and given to both launcher roots as a context
  beside `surface_root`'s. The daemon writes it for `sill launcher toggle|open|close` and for
  the panel's `KeyboardFocus`; the panel writes it for Escape and activations, the catcher for a
  press. Each surface follows it onto its own `SurfaceOp::SetVisibility` (`follow.rs`), so
  neither needs the other's id. A focus loss closes only a panel that has held focus since it
  opened (the leave of the previous opening's unmap can arrive after the next toggle). The
  launcher opens on the first output that has one: the daemon does not know which output the
  pointer is on (design/13 §13.3.9 "the output under the pointer" is not met). Daemon lines
  outside "spawn and route": the `run_event` parameter, the `KeyboardFocus` check, and
  `collect_stats` reporting `sill-launcher-<output>` and `sill-launcher-catcher-<output>`
  (the acceptance needs their frame counts), as the dock's F100.
- **F141 The panel is quire's `CommandPalette`, as it is.** The palette owns its selection,
  its Up/Down/Enter/Escape and its field's focus, and draws inside whatever surface hosts it:
  its `--scrim` wrap fills the 680 x 460 panel surface and its card is 540 wide, 11 % of the
  width down (a dim rectangle around the card, visible in the screenshots; Q40). The keys it
  leaves alone reach the surface's root `div#panel`, which every key event reaches (bubbling
  from the field, or targeted at the root with no focus): Tab and Shift+Tab step the frozen
  machine's `Filter` (providers with no rows are skipped; the palette keeps its selection,
  clamped, as the machine does), Ctrl+K and Alt+K open the machine's selected row's actions as
  a `Menu{Rich}` anchored at a fixed point over the card (the palette reports no row rects,
  Q41). The machine mirrors Up and Down to know that row; a selection the pointer moves is not
  seen (Q41). `ResultsList` and `ActionsList` (frozen names, no handlers in their props) render
  the same palette and menu inert, for pictures. Material is `launcher.material` (Sheet by
  default) on a `Ds` root with the active workspace's `SpaceLook` and the environment's tint;
  blur is the whole panel (`id="panel"`, radius 14), the card has no id of its own (Q40).
  Grouping (`groups.rs`) agrees with launcher-core's arranged order (F114): typed text is the
  Top hit then providers in Tab order, the empty query System rows as Actions then five as
  Recent; a Tab filter shows one provider's rows. Row hints (`entries.rs`): `↵` on the row
  Enter runs, `⌃K` on rows with actions, the kind as a note in mixed groups. The empty state
  is `Nothing matches "<query>".` (the ds Menu's words; design/06 §11.3's text is mailo's).
- **F142 `keys::step` and the actions menu.** `step` knows counts, not ids, so an Enter inside
  the actions menu cannot name the action: `step` closes the menu there and emits nothing, and
  the pick comes from the ds `Menu`'s `onpick`. Likewise Enter in the results is the palette's
  (`onpick` carries the row's identity, which is safer than an index when a batch lands between
  the key and the pick); the machine's `Activate` output is table-tested but not wired.
- **F143 Activation.** A pick sends `SearchCommand::Chosen` first (the runner looks the row up
  among the current rows and carries out `FocusWindow`, `CloseWindow` and `Launch`, F117), then
  asks the row's provider (`LauncherProviders`, the five real ones built from `Services`, or a
  demo's fakes) for its `Activation` and does the rest: `Copy` through
  `SurfaceHandle::copy_text`, `Insert` into the field (the launcher stays), `OpenSettings`
  `Cosmic` launches COSMIC Settings with an activation token taken from the panel before it
  hides. Then the launcher closes, which sends `Cancel`. The palette's `onclose` comes just
  before its `onpick`, so a dismissal is deferred a turn and skipped when a pick follows.
  Every keystroke is `SearchCommand::Query` (the runner's generation); rows of the generation
  the launcher opened over, or older, are not shown (`groups::fresh`).
- **F144 Warm surfaces and latency.** Both surfaces are spawned at daemon start (catcher, then
  panel) and hidden before their first paint; `set_visibility` shows and hides them (no
  respawn). shell-host unmaps by destroying and rebuilding the layer surface, so the GPU
  presenter is recreated at every show and the catcher is "painted once" per showing. The
  panel logs `launcher.open_to_frame_ms=` (request to the first frame callback of the shown
  panel, through a held `FrameDrive`); perf-ime's `launcher.open_latency_ms` (F102) times only
  the IPC dispatch, which never waits for the frame. The catcher's input region is the output
  less the panel's rectangle (`layer::catcher_input`), so a press on the panel reaches the panel
  whichever Overlay surface is stacked on top.
- **F145 Mounting policy, forced by Q43 and Q45.** A new palette mounts `MOUNT_WAIT` (60 ms)
  after an opening, when its first results have usually landed (mounting during a results
  re-render is when Q43 fires most); until then the previous palette shows the new opening's
  state. A palette (and the actions menu) is never unmounted within `ENTRANCE_GUARD` (900 ms)
  of mounting (Q45): a quick reopening reuses it (no entrance), the refocus after the actions
  menu waits, and the palette stays mounted while the launcher is hidden. The field gets the
  keyboard back after the actions menu only by a remount (Q44).
- **F146 What an activation cannot reach yet.** `Services` has no session control, so
  `Activation::System` (lock, log out, suspend, restart, shut down, reload) is logged and
  dropped; `SettingsTarget::Key` has no settings app; COSMIC Settings' `Exec` takes no page, so
  `SettingsTarget::Cosmic(page)` opens its default page.
- **F147 IME.** The field is quire's `TextInput`, a Blitz text input; shell-host feeds it
  `zwp_text_input_v3` preedit and commit as Blitz IME events (`ime.rs`), and Blitz's input sets
  the preedit as parley's compose region, which it draws underlined, and inserts the commit.
  Not verified live: the nested session has no input method, and `ds_native::Harness` has no
  IME event to send (perf-ime owns the KWin CJK report, F104).
- **F148 Tests.** `tests/launcher_lint.rs` (the stylesheet under Strict; the panel's markup
  from the harness, since an SSR pass does not mount the floating palette; the SSR root and the
  catcher), `tests/launcher_render.rs` (groups, the top hit selected with `↵`, `⌃K`, Actions and
  Recent, the empty state, a calculation) and `tests/launcher_keys.rs` (Down, Down, Enter runs
  the third row; Tab narrows to Applications, then Windows, and Enter still runs from the field;
  Escape closes; Enter launches the top hit) drive the real `Launcher` in a
  `ds_native::Harness` over a fake runner that answers queries with generations and carries out
  chosen rows as F117 does. `ds-native` is a new dev-dependency (a sibling path, beside `ds`;
  root `Cargo.toml` and `sill-surfaces/Cargo.toml`, not files this wave owns). Each harness test
  runs again when it dies of Q43 (up to 20 times); in parallel runs the race hits often.
- **F149 Acceptance** (`dev/accept-launcher-ui.sh`, nested cosmic-comp on the running
  `wayland-1` after its restart on Mesa, scratch config). With the daemon built `--release
  --features debug` (`SILL_BIN=target/release/sill`) all five steps pass: (1) both launcher
  surfaces are in `sill debug stats` from the start and the panel cannot be captured (hidden);
  (2) `sill launcher toggle` to a capture of `sill-launcher-WINIT-0` holding the panel: first
  opening 106 ms, 10 warm openings min 52, p50 57, max 80 ms; the panel's
  `launcher.open_to_frame_ms` read 105, then 54-79 ms; (3) a second toggle hides it and a click
  on `sill-launcher-catcher-WINIT-0` closes a reopened one; (4) 5 s idle after the hide: 0
  frames on the panel and on the catcher; (5) on `examples/launcher_demo.rs` (fake runner;
  typing and Ctrl+K on its stdin, G20): `open`, `type fi` shows the grouped rows, `key actions`
  opens Firefox's actions (20014 pixels change), two `key escape` close the menu then the
  launcher. With the debug daemon step 2 fails: warm openings take 200-240 ms (the first frame
  of a debug-built Blitz render), the rest pass. A 15-opening stress on the debug daemon,
  toggles 150 ms apart included, did not crash after F145 (before it, Q45 killed the daemon).
  Screenshots: `dev/out/launcher-cosmic.png` (open over "fi"),
  `dev/out/launcher-cosmic-actions.png` (Firefox's actions menu). The catcher is sized by the
  viewport (`100vw` x `100vh`): an empty absolutely placed box with `inset:0` got no hit area on
  Blitz and the catcher's press never arrived (`tests/launcher_catcher.rs`).

### Gaps found in M4 launcher-ui

- **Q40** `CommandPalette` has no embedded mode: it always draws its scrim wrap over the whole
  host and a `min(540px, 88%)` card placed 11 % down, so a shell panel surface shows a dimmed
  rectangle around a card smaller than the surface, and the card carries no id for a blur
  region. Its entrance is fixed (`peek-in`); `cmdk-in` is not selectable.
- **Q41** `CommandPalette` exposes neither its selection (no `selected`/`on_select`) nor its
  rows' rects, and passes no key through: Tab and Ctrl+K are read at the surface root and the
  selection is mirrored from Up/Down, which misses a pointer-moved selection.
- **Q42** `Tile` (a palette row's tile) takes a glyph, text or avatar, not an `IconSource`, so
  app icon files cannot be drawn in the launcher; rows show their provider's glyph.
- **Q43** `TextInput { focus: Focus::OnMount }` calls `MountedData::set_focus` from a spawned
  task; dioxus-core polls a task woken in the same turn as a dirty scope inside
  `render_immediate`, where dioxus-native-dom's writer holds the document, and `set_focus`
  borrows it mutably: "RefCell already borrowed" (dioxus-native-dom `events.rs:161`), the rect
  race quire fixed with `HostMeasure`, now for focus. In `ds_native::Harness` it panics often;
  live (shell-host polls the same way) it did not show in 25 openings and 25 Tabs, but it
  would take the daemon down.
- **Q44** No way to give a field focus again: no focus prop or handle on `SearchField`, so after
  a ds `Menu` takes the keyboard the palette must be remounted (replaying its entrance).
- **Q45** `MotionTimer` (`ds/src/motion/timer.rs`) spawns its settle task with `spawn_in(scope)`
  and sets its signals after the sleep; when the component unmounts first, `task.set(None)`
  panics ("ValueDroppedError", line 50) and takes the process down. Found live: the daemon
  died on a `sill launcher toggle` pair 0.5 s apart, which unmounted a palette mid `peek-in`.
  Any `use_entrance` user (palette, menu, popover) unmounted early is exposed.
- **G20** shell-host's `debug-inject` has pointer and click but no key, so `sill debug` cannot
  type or press Escape/Ctrl+K; the demo takes those on stdin through a `LauncherScript` context
  only it provides (`script.rs`).

## Adoption of the launcher gaps (2026-09-24)

quire master merged `launcher-gaps` (sill Q40-Q45, and the dock's Q15-Q17; quire FINDINGS
"Launcher gaps"). This pass removes the launcher's and the dock's workarounds for them, in the
order of quire's "what sill changes" list. Numbers start at F160/Q60 so they stay clear of the
wallpaper wave running beside it.

- **F160 The crash guards are gone.** `mounting.rs` (`ENTRANCE_GUARD`, `PaletteMount`, the
  900 ms keep-mounted rule for the palette and the actions menu) and `MOUNT_WAIT` (the 60 ms
  delayed mount) are deleted. The palette mounts with each opening, keyed by the opening's
  serial, and unmounts when the launcher hides; the actions menu mounts and unmounts with the
  session's `ActionsMenu`. `daemon::surface_root` gives every root `ds_native::focus::FOCUS`
  beside `MEASURE` (re-exported as `sill_surfaces::FOCUS`, since the `sill` crate does not name
  `ds-native`; one line in `lib.rs`), the dock's context menu popup calls
  `ds_native::focus::provide()`, and both demos add the context. The harness's
  `retrying_the_focus_race` is deleted: every launcher test runs once, and a full
  `cargo test --workspace` passed with no retries.
- **F161 The field gets the keyboard back without a remount.** The panel holds one
  `use_focus_request()`, passes it as the palette's `focus`, and calls `request()` when the
  actions menu closes (its `onclose`, or the machine's Escape/Ctrl+K from the demo's script).
  `tests/launcher_actions.rs` opens Files' actions, presses Escape, and finds the menu gone,
  `input` focused, the card still `data-presence="present"` and Files still selected; Enter then
  launches Files.
- **F162 Selection, keys and the menu's anchor come from the palette.** The palette's
  selection is the session's (`selected: Some(keys.selected)`); Up, Down and the pointer ask
  through `on_select`, so a hovered row is the one Enter runs and Ctrl+K opens
  (`the_row_under_the_pointer_is_the_selection`). Tab, Shift+Tab, Ctrl+K and Alt+K are read
  from the palette's `onkey`. The root `div#panel` keeps one `onkeydown`, which only calls
  `prevent_default` on those four keys as they bubble past: `onkey` hands on an owned
  `KeyboardData`, and Blitz's default for Tab moves the focus to the next node. The harness
  tests pass without it too, so whether it is needed live is unproven (Q61). The actions menu
  hangs from `Anchor::Rect` of the row rect the palette reports (`ACTIONS_AT` is gone; the inert
  `ActionsList` picture keeps a private point). `ResultsList` now shows its `selected` prop.
- **F163 The palette is embedded.** `host: CommandPaletteHost::Surface`,
  `entrance: PaletteEntrance::CmdkIn`, `id: "launcher-card"` (`layer::CARD_ID`, now the panel's
  blur region at radius 14), inside a body sized 680 x 460 from the layer constants. The dim
  box around the card is gone: the card is the panel (`dev/out/launcher-cosmic.png`).
- **F164 A new palette's first row rect is reported empty (Q60).** Live, the palette mounts
  as the surface is mapped again and reads its selected row before the first layout: it reports
  a 0 x 0 rect at the origin, and because neither the selection nor the row element under it
  changes after that, it never reports again. The first actions menu then hung at the
  viewport's 8 px clamp, over the field (seen in the first acceptance run's screenshot; the
  harness, which lays out first, measured 6, 80.5, 668 x 45.1). The panel treats an empty rect
  as unknown (`panel::measured`) and hangs the menu from `FIRST_ROW_FOOT` then: the first row's
  foot, which is the only row selected before the selection moves. Fixing this belongs in quire
  (measure again after the first layout, or on a zero read).
- **F165 App icons in rows.** `entries::tile`: an `IconRef::File` (the index's resolved icon)
  is `Tile::Source(IconSource::Image(.. IconSize::Tile48))`, or `Symbolic` for a `*-symbolic`
  file; otherwise the named glyph, otherwise the provider's glyph. The launcher's fake rows
  (tests and demo) carry the hicolor 48 px SVG of an app when the system has one, so COSMIC
  Files and Terminal draw their icons in `dev/out/launcher-cosmic.png`. Firefox keeps the
  provider glyph: this system has no Firefox icon.
- **F166 Dock.** The pill root takes `radius: Some(Corner::Px(Px(dock.pill_radius_px)))`, so
  the setting reaches the pill (Q15 closed). A tile's icon is drawn at its share of the plate
  in whole pixels through `IconSize::Px` (72 % for an icon file, 56 % for a glyph) at every
  magnified size; `DRAWN`, `icon_scale` and the `transform:scale` are gone (Q16 closed). The
  hover label is always in the document as `Tooltip { shown }`: `Visible` for the tile the
  machine's `Label::Shown` names, `Hidden` otherwise. The dock provides that tile as a
  `Signal<LabelShown>` context, and the frozen `Tile` props are unchanged. The fly delay, the
  warm skip and the hide on press, drag and open menu are now the machine's, not the Fly's
  `:hover` (Q17 closed). The `LabelGate` re-export is removed; the machine still uses the gate
  itself.
- **F167 Acceptance** (nested cosmic-comp on the running `wayland-1`, 2176 x 1360; the host
  was loaded throughout, load average 42-50, with `hu200` on about 28 of 32 cores).
  `dev/accept-dock.sh` (debug daemon): all six pass. Maximised 1256; a sweep drew 35 frames for
  34 callbacks, the shrink 8, then 0 in 5 s idle; the running dot after 1691 ms; the badge red
  after 823 ms; the context menu popup opened. The new step 6 writes
  `dock.pill_radius_px = 12` to the scratch config and finds the pixel 4 px inside the pill's
  corner to be pill (alpha 150). The same step run with `PILL_RADIUS=22` finds it empty
  (alpha 6), so the check tells the two apart. `dev/accept-launcher-ui.sh` with the
  `--release --features debug` daemon: warm, hide, idle, the new stress step and the demo pass;
  the latency step fails. First opening 149 ms; 10 warm openings min 83, p50 117, max 148 ms;
  the panel's `open_to_frame_ms` 85-145. The demo's actions menu hangs under the Firefox row
  (37407 px changed). Stress: 20 toggles 150 ms apart (3457 ms), then 5 s idle: the daemon
  is still running, answers `sill ping`, and its log has no panic (every close lands mid
  `cmdk-in`, which is what Q45 used to kill). The launcher alone cost less than the load: master's
  release daemon (built against the same quire) and this branch's, alternated twice under the
  same load with 10 warm openings each, gave `open_to_frame_ms` p50 130/120 ms for master and
  135/147 ms for the branch (min 92/83 vs 105/107). Mounting a palette at every opening costs
  about 15-20 ms. The panel's slowest frame is now 49 ms of work (p50 2 ms), and the frame
  that mounts the palette is the slow one. F149's 57 ms p50 was measured on a quieter host.
  Row icons are not the cost: with them switched off the numbers were the same. The demo step's
  waits are longer now (2.5 s after `open`, 2 s after `type`): under this load the debug
  demo's first frame took 1.1 s, and a `type` that lands before the opening's own session reset
  is lost. Screenshots: `dev/out/launcher-cosmic.png`, `launcher-cosmic-actions.png`,
  `dock-cosmic.png`, `dock-cosmic-magnified.png`, `dock-cosmic-menu.png` (pill at 12 px).

### Gaps left after the adoption

- **Q60** (quire) `CommandPalette`'s `on_select_rect` reports 0 x 0 for a row read before the
  surface's first layout and does not measure again (F164). sill hangs the menu from a
  fallback point until quire fixes this.
- **Q61** (quire) `onkey` gets an owned `KeyboardData`, so the caller cannot
  `prevent_default` the Blitz default of a key it takes (Tab moves the focus). sill prevents it
  at its root; whether that is needed live is not proven (F162).
- **Q62** Ctrl+K or Alt+K while the actions menu is open does not close it: the menu has the
  keyboard and its keys do not reach the palette's `onkey`. Escape closes it; design/06 §20.2
  says Ctrl+K again should too. The demo's scripted Ctrl+K still toggles it.
- **Q63** Mounting on every opening puts the warm opening above the 100 ms gate on this loaded
  host (F167; about 15-20 ms over master under the same load). Keeping a palette mounted while
  hidden is now safe (Q45 is fixed) and would win that time back at the cost of the entrance
  replay; that is a design decision, left open here.
- **Q64** (shell-host) While shown, the panel redraws at about 4 ms intervals during the
  `cmdk-in` entrance (Mailbox present mode, `interval` p50 4 ms, about 65 frames per 0.4 s
  opening), and one bench session logged `watchdog_fired: 1` on the panel. The panel is still
  idle once hidden (0 frames in 5 s).
- **Q65** The bar's menu popup and the tray's dbusmenu popup do not yet call
  `ds_native::focus::provide()` (not files this pass owns). Nothing crashes without it; a
  collision prints the caught panic.

## Adoption of the polish pass (2026-09-24)

Branch `adopt-polish`, against quire `v0.1.1` (465ae77) and then `v0.1.2` (7a7ae45, through
master's merge). What sill changed for quire's macOS polish pass (quire FINDINGS "macOS
polish", CONSUMING.md "The macOS polish pass"):

- **Settings.** `sill-settings` registers the new keys under their domains with quire's
  defaults: `bar.item_font_px` (13), `bar.item_font_weight` (`Count`, 500, 100..=900),
  `bar.item_radius_px` (4); `menus.font_px` (13), `menus.highlight_radius_px` (6),
  `menus.tooltip_font_px` (12); `launcher.field_font_px` (22), `launcher.field_font_weight`
  (500), `launcher.field_glyph_px` (20), `launcher.row_title_px` (14), `launcher.row_detail_px`
  (12); `dock.pill_padding_px` (6), `dock.running_dot_gap_px` (3), `dock.floor`
  (`DockFloor::{Off,On}`, Off). Three defaults move: `menus.item_height_px` 24 to 22,
  `menus.separator_margin_px` 4 to 5, `dock.tile_gap_px` 4 to 8. The fourteen new keys have no
  design/22 §3 row yet (design/22 is quire's): `tests/keys.rs` lists them in `AWAITING_ROWS`,
  exempt from the row check only while the row is missing. `sill --write-schema` writes 156 keys.
- **Shell-wide inputs** (`surfaces/bar/shell_style.rs`). `shell_metrics` builds
  `ds::ShellMetrics` from `bar.*`, `menus.*` and `launcher.*`; every surface writes its
  `style_attr()` on a box around its `Ds` root (`.sill-bar-root`, `#dock-pill`, `#panel`) and
  every popup on its frame (the popup looks carry the metrics), so the 22 px menu rows, the
  5 px separator margins and the launcher's 22/500 query follow the keys. `material_stack`
  builds `ds::MaterialStack` and every root takes it as `Ds { stack }`. `AppearanceSettings`
  has no `material_*` fields yet, so the six `appearance.material_*` keys are read from its
  `extra` table (percent, clamped, each falling back to its default alone).
- **Bar.** The app name (`Emphasis::Strong`) and the clock sit in `MenuBarItem { open }`,
  which draws the hover and open pill; the title slot keeps only the hit box and events, and
  its own `data-open` pill and padding are gone. The workspace indicator is one
  `WorkspacePills` group of `WorkspacePill`s (`Here::Current` for the active one; a primary
  press activates), each in the wrapper that keeps the drag-to-reorder.
- **Dock.** The pill's corner is `Corner::Squircle(dock.pill_radius_px)` and the blur region's
  radius is the squircle's shadow circle (`pill_blur_radius`: .884 r, 19 at 22). `DockMetrics`
  from the keys (`geometry::dock_metrics`) is written around the root, whose first child is
  `DockFloor`. The padding is `dock.pill_padding_px` everywhere the constant `PAD_PX` was
  (rows, layer, magnify params, the slide distance): the layer is 158 tall and reserves 68.
  Every tile is `IconView { plate: Some(PlateFamily::Neutral) }` (design/08 §4.1: third-party
  icons and the §4.3 fallback glyph on the paper plate; quire sizes the glyph 56 % and an image
  72 %), replacing sill's own rounded-rect plate. The running dot is quire's `RunningDot` in a
  box at the tile's rest place, so it stays on the baseline while the tile bounces.
- **Launcher.** The card is `CommandPalette { corner: Some(Corner::Squircle(14)) }` and content
  sized. An empty foot element after it in the panel's column is measured through the host's
  `HostMeasure` a frame after anything that changes the card (opening, query, row count, actions
  menu), and the panel publishes `PanelHit::{Whole, Card { height }}` on the `LauncherLink`
  (`launcher/card.rs`, `launcher/hit.rs`). The panel's input follows it (the card element, or
  the panel's 680 x 460 while the actions menu is open, since the menu can hang below the
  card), the blur is the card element at radius 12, and the catcher leaves out only the card, so
  a press under a short card closes the launcher. The surface gained room for the Sheet's
  ambient shadow (`0 24px 60px -18px`): 42 px each side and 66 px below; the card keeps its
  place on the output. `LauncherLink` is now a struct of two channels.

### Acceptance (nested cosmic-comp, 2176 x 1360 physical, 2000 x 1250 logical at the shots)

The scripts seed COSMIC's background record in their scratch `XDG_CONFIG_HOME` with a
generated test wallpaper (`dev/backdrop.py`: 2176 x 1360 soft colour bands; `dev/backdrop.sh`).
m2-wallpaper had not merged, so `cosmic-bg` drew it in the nested session for every shot;
once `sill run` has a wallpaper surface the same record is what it draws, and `backdrop_up`
skips `cosmic-bg`. `backdrop_by_sill` pings before `debug stats`: any other sill request
starts a daemon when none runs (`client::send`), which left a stray daemon behind the first
attempt and made the next script refuse to start.

- `dev/accept-dock.sh`: all 6 PASS. Maximised probe 1260 of 1360 (bar 32, dock 68); sweep 39
  frames for 38 callbacks, 0 frames in 5 s idle; the launched Terminal's dot after 602 ms;
  the badge after 152 ms; the context menu popup opens; the squircle corner probe (6.5, 1.5 px
  inside the corner) is the pill at `pill_radius_px = 12` (alpha 211) and outside it at 22
  (`PILL_RADIUS=22`, alpha 5), so the probe tells the two squircles apart.
- `dev/accept-bar.sh`: all 8 PASS (idle 0 commits, menu opens and dismisses, reserve 68).
- `dev/accept-launcher-ui.sh` on the release build: all 6 PASS. Warm openings min 64, p50 68,
  max 100 ms (first 153); daemon `open_to_frame_ms` 65-99 warm; idle 0 frames on both
  surfaces; 20 toggles, no panic. Under a load average of 18 an earlier run measured p50 151,
  max 540 and failed the 100 ms gate; the same build passed once the load fell to 8.
- Screenshots: `dev/out/bar-cosmic.png`, `bar-cosmic-menu.png`, `dock-cosmic.png`,
  `dock-cosmic-magnified.png`, `dock-cosmic-menu.png`, `launcher-cosmic.png`,
  `launcher-cosmic-actions.png`.

### Gaps left

- **Q66** design/22 §3 has no rows for the fourteen new keys (the row list is in the handback);
  until they land, `AWAITING_ROWS` exempts them. The three moved defaults' rows still say 24,
  4 and 4.
- **Q67** `appearance.material_*` are read from `AppearanceSettings::extra`; once ds-settings
  registers them as fields, `material_stack` must read the fields (the `extra` lookup then
  silently finds nothing and falls back to the defaults).
- **Q68** `dock.pill_radius_px` stays 22 (quire recommends 18 for a squircle). At the 60 px pill
  both extents are capped at 30, so only the shadow circle differs (19 vs 16).
- **Q69** The bar and dock popups keep 24 px of side room; the menus' ambient shadow reaches
  28 px, so its last, near-transparent edge is cut (faintly visible over a light wallpaper in
  `dock-cosmic-menu.png`).
- **Q70** `sill-services`' `MagnifyParams::design_defaults` still holds design/10's gap 4 and
  pad 8; the dock's test compares against it with the two fields overridden.
- Resolved after the merge: Q66 and Q67 by the design/22 rows and ds-settings' own material
  fields (sill reads `Environment::material_stack`); Q69 by 28 px of side room in both popups'
  geometry; Q70 by `design_defaults` moving to gap 8 and pad 6 (the layout tests' worked numbers
  keep design/10's first spacing through `first_spacing`). Q68 stays at 22: at the 60 px pill
  the corner curve is the same as 18, and 22 matches quire's `DockMetrics` default.

## M2 wallpaper (2026-09-24)

Filled `wallpaper::spawn` and the `Wallpaper` component; the frozen types in
`sill-services/src/wallpaper/mod.rs` (`WallpaperSource`, `Wallpapers`, `WallpaperImage`,
`WallpaperCommand`) and `cache_file_name` needed no change.

- **F131 The source is COSMIC's `all` record only, read by hand.** `ron` is not in the pinned
  dependency block (CONVENTIONS §11: a new dependency is a change to `docs/workspace-deps.toml`,
  reviewed first, not added to one crate alone), so `config::parse_source_path` reads the one
  field it needs — `source: Path("...")` — out of the RON text directly rather than
  deserialising the record, unescaping RON's string escapes by hand. This machine has
  `cosmic-bg` 1.8.0 installed (`rpm -q cosmic-bg`; not the desktop in use, but the package is
  present), whose real default record at
  `/usr/share/cosmic/com.system76.CosmicBackground/v1/all` is exactly the shape this reads:
  ```
  (
      output: "all",
      source: Path("/usr/share/backgrounds/default-dark.jxl"),
      filter_by_theme: #true,
      rotation_frequency: 3600,
      filter_method: Lanczos,
      scaling_mode: Zoom,
      sampling_method: Alphanumeric,
  )
  ```
  `cosmic-bg`'s schema carries one `source` per record, not a light/dark pair, so
  `WallpaperSource::Pair` is never produced by this reader — only `File` (one path, both
  schemes) or `None`. `config.rs`'s tests pin this exact record text as a fixture, plus a
  `Color(..)` source (no path) and a missing `source:` field, both answering `None`.
- **F132 COSMIC's own default background is a format this crate's `image` build cannot decode.**
  The pinned `image = { features = ["png", "jpeg"] }` has no `jxl` feature, and cosmic-bg's
  stock backgrounds on this machine are all `.jxl` (`/usr/share/backgrounds/default.jxl` and
  `default-dark.jxl`, both symlinks into `f44/default/`); `/usr/share/backgrounds/cosmic/*.jpg`
  and `*.png` decode fine. `resize::ensure_cached` answers `Err` for an undecodable source
  (`image::open`'s own error, wrapped), and `wallpaper::publish` treats that exactly like
  `WallpaperSource::None`: the output's entry is dropped, so the surface falls back to
  `--paper` rather than showing a stale or partial image. Not a bug to fix here — a real COSMIC
  session with a PNG/JPEG background works today — but worth a install-time note if `sill`
  ever ships a `dist/` default: point it at one of the `cosmic/*.jpg` backgrounds, not
  `default.jxl`.
- **F133 The cache hash is FNV-1a, not `DefaultHasher`.** `cache.rs`'s own doc names the reason
  (`DefaultHasher`'s `RandomState` seed changes every process, so the same source would get a
  new cache name on every restart — defeating the cache); `source_hash` feeds the path's raw
  bytes, the length and the modification time (nanoseconds since the epoch) through one FNV-1a
  accumulator. `resize::ensure_cached` checks `cached.is_file()` before touching `image` at
  all, so "renders only on change" holds at the cache layer too: a second `Prepare` for the
  same source and size never re-encodes (`resize.rs`'s own round-trip test asserts the cache
  file's mtime is unchanged the second time).
- **F134 Cover-fit resize, then crop — not resize-to-cover in one call.** `geometry::cover_fit`
  is pure: given a source and a target size, it picks a `scaled` size (Lanczos3 target) that is
  never smaller than the target on either axis, and a centred `(crop_x, crop_y)`. `resize.rs`
  then calls `image::resize_exact` to `scaled` and `crop_imm` to the target, writing the result
  to a sibling `.png.tmp` file and renaming it over the final name — a reader of the `file://`
  URL (shell-host's `LocalNet`, `dom/net.rs`'s own module doc: "the wallpaper is
  `background-image: url(file:///…)`") never sees a half-written PNG. The crop is symmetric to
  within one pixel of an odd overflow (`geometry.rs`'s own sweep test), which is what makes
  "a capture's centre pixel matches the source's" a meaningful acceptance check rather than a
  coincidence: cover-fit *by construction* keeps the source's own centre aligned to the
  target's.
- **F135 The COSMIC config watch degrades to "read once" rather than failing to start.**
  `watch::watch` needs `notify` to watch an existing directory; COSMIC's config tree may not
  exist at all outside a real COSMIC session (it does not on this dev machine before the
  acceptance script creates it). `nearest_existing_ancestor` walks up from the `all` file's
  parent to the first directory that exists and watches that one recursively, so a directory
  created later (COSMIC's own lazy write) is still caught; when nothing above it exists either
  (no `$HOME`), `spawn` simply runs with no live watch — `config::resolve_source`'s read at
  start time still stands, it just never updates without a restart or `WallpaperCommand::Reload`.
  Debounce is 500 ms (the brief's own number), distinct from `sill-settings::watch`'s 30 ms: a
  foreign config file is edited far less often than the shell's own, written by cosmic-bg on a
  human's own "change background" click, not a live-typing settings field.
- **F136 `watch::watch` takes `&Handle`, not the bare `tokio::spawn`.** The first version called
  `tokio::spawn` directly inside `watch()`, which needs an entered runtime on the *calling*
  thread — true inside an `async fn` already running on a runtime, false for `Services::spawn`
  called from a plain thread holding only a `Handle` (exactly what
  `services::tests::spawn_does_not_panic_and_every_stub_holds_its_default` does, and what the
  real daemon does too: `crates/sill/src/daemon.rs::run` calls `Services::spawn(runtime.handle())`
  before it ever calls `runtime.enter()`, which happens only later and only around
  `sill_settings::watch`). The test caught it immediately ("there is no reactor running, must be
  called from the context of a Tokio 1.x runtime"); every other service's `spawn` in this crate
  already gets this right by using the passed `rt: &Handle` (`clock::spawn`'s `rt.spawn(run(..))`
  is the precedent) — `watch::watch` now does the same.
- **F137 `sill run` does not yet spawn a wallpaper surface.** `crates/sill/src/daemon.rs`'s
  `spawn_missing_bars`/`spawn_bar` are bar-specific; `outputs.rs`'s `OutputSurfaces.wallpaper`
  field and `OutputAction::Resize`'s own doc comment ("re-apply sizes, re-prepare the wallpaper")
  already anticipate a wallpaper surface, but nothing calls `wallpaper_layer` or sends
  `WallpaperCommand::Prepare` from the daemon yet. `daemon.rs` is frozen to every wave but
  "daemon" (DESIGN.md "Wave ownership"), so this is reported rather than patched here. Worked
  around the same way `dev/accept-bar.sh`'s menus step routes around a daemon/host gap of its
  own (G8, through `accept-bar-ui.sh cosmic`): `dev/accept-wallpaper.sh` runs the real
  `sill_services::wallpaper::spawn` and the real `Wallpaper` component through its own harness,
  `examples/wallpaper_demo.rs`, with everything else faked; its first step still runs against a
  live `sill run` when one exists and prints whether `sill debug stats` shows a
  `sill-wallpaper-*` surface, so the gap is visible on every run.
- **F138 A concurrent quire merge broke `sill-surfaces`' bar code mid-wave; fixed by merging
  master.** quire commit `f3c1115` ("Merge tray-gaps: external icons, submenus and disabled
  rows, element ids and pointer buttons", landed 2026-09-24 11:30, closing this repo's own
  Q6-Q8) changed `ds::Button` and `ds::IconButton`'s `onclick` from `EventHandler<()>` to
  `EventHandler<Press>` and added `MenuEntry::Item.availability` and a new `MenuEntry::Submenu`
  variant. None of this wave's own files reference any of it — `cargo build -p sill-surfaces
  --lib` failed with seven errors, all in
  `surfaces/bar/{app_name,clock,tray,workspaces,content,status_item,geometry}.rs`, none under
  `surfaces/wallpaper/` — but `ds` is a path dependency shared by every sill worktree (`../quire`
  resolves to the same on-disk checkout everywhere), so the whole `sill-surfaces` crate stopped
  compiling partway through this wave's own verification, after every wallpaper test had already
  passed once against the pre-merge quire (recorded in full in this wave's own report) and
  before `dev/accept-wallpaper.sh`'s own build of `wallpaper_demo` could run. `bar/**` is frozen
  to this wave, so rather than patching it directly this wave merged `master` (which by then
  carried `60582f4`, "The bar adopts quire's typed presses and the availability field on menu
  items, and sizes submenu rows," plus the merged apps-index, launch, pins and badges services)
  into `m2-wallpaper`; the merge touched no wallpaper file (`git merge master` only conflicted on
  this file, `FINDINGS.md`, over both sides appending new sections).

### Gaps found in M2 wallpaper

- **Q14** `ds::Material` has no variant for "paints nothing" (design/20 §1.4: "Material: none
  (opaque image)"); every material's root still paints `background: var(--m-tint-solid)`
  (`materials_css.rs`) even though nothing shows it. `Wallpaper` uses `Material::Widget` with
  `BlurState::Unavailable` — the closest kind (a background-level surface, not a bar, menu or
  sheet) — since the wallpaper's own opaque layers (or, with no image, `.sill-wallpaper`'s own
  `--paper`) cover the root completely regardless of which material is picked; `Material::Window`
  is the one genuinely wrong choice, since its `--m-tint-solid` is the Space's own gradient
  (`var(--f-grad)`), exactly the per-workspace tint design/21 §8 says the wallpaper must not
  carry.
- **F139 A second merge, and `MEASURE` on the demo's own root.** Master moved again while this
  wave was verifying (`d3ffa0c`, a fix to the bar's own menu test for the new `Submenu` variant,
  and the M2 freeze amendment above): `cargo test --workspace` failed with the same
  non-exhaustive-match shape as F138, this time in `surfaces/bar/content.rs`'s test helper —
  fixed upstream by `d3ffa0c` before this wave ever touched that file. Merging `master` again
  (now at `f10e63d`) picked that up; again no wallpaper file changed, only `FINDINGS.md`
  conflicted (both sides appending). That merge also brought F81's `sill_surfaces::MEASURE`: a
  popup or a demo that builds its own `Root` must add `.context(sill_surfaces::MEASURE)` itself
  (the daemon does this for every surface it spawns, but `examples/wallpaper_demo.rs` builds its
  own), so `spawn_wallpaper` in the demo now does too — without it, `Ds`'s `OverlayHost` panics
  ("RefCell already borrowed") the first time the renderer polls a woken task inside its own
  render pass (F81's own wording).- **F168 The daemon spawns a `Wallpaper` per output, added at the coordinator's request (the
  wave's own scope was reopened to exactly this).** `daemon.rs` gained `spawn_missing_wallpapers`/
  `spawn_wallpaper`, mirroring `spawn_bar`/`spawn_missing_bars` line for line: on each reconcile
  it sends `WallpaperCommand::Prepare` at the output's buffer size (`shell_host::scale::
  buffer_size`), skipped for an unnamed output (no `sill_services::OutputName` to key it by, the
  same restraint the bar's own `on = output.name.clone().map(OutputName)` shows), then spawns
  the layer through the daemon's shared `surface_root` helper and fills
  `OutputSurfaces.wallpaper`. Closing on `OutputRemoved` needed no new code at all:
  `OutputSurfaces::ids()`/`close_every` already iterate that field generically, so it started
  working the moment the field was filled.
- **F169 `sill debug stats` was bar-only; fixed alongside dock-ui's own identical fix.**
  `collect_stats` filtered on `surfaces.bar` alone, so a spawned wallpaper surface never showed
  up in `sill debug stats` — `dev/accept-wallpaper.sh`'s own discovery step (which finds a
  surface's name from that reply before capturing it) failed with "no wallpaper surface to
  capture" even though the surface existed and had presented frames. `outputs::
  wallpaper_namespace` (mirrors `bar_namespace`, own unit test) plus a `.chain(..)` onto
  `collect_stats`'s iterator fixed it. Merging master afterwards found dock-ui had independently
  reached for the exact same shape (`dock_namespace`, `flat_map`) for the dock, and M4
  launcher-ui a third time for the launcher panel and catcher — `collect_stats` now chains all
  five surface kinds together; every one of the four fixes came from a different wave without
  coordination and merged into one function with no further disagreement once the shapes were
  compared.
- **F170 Four merges of `master` landed inside this one wave, and a compositor restart.** In
  order: (1) F90/F91's quire Press/`MEASURE` churn; (2) shell-host's `Inject::Click{at,button}`
  (a mechanical `to_host_button` addition, no conflict); (3) `adopt-bar-gaps`/`m4-providers-fix`
  (the bar's new `MenuEntry::Info` match arm, and dock-ui's own `spawn_missing_docks`/
  `dock_namespace` interleaved with this wave's wallpaper equivalents in both `daemon.rs` and
  `FINDINGS.md`); (4) `adopt-launcher-gaps`/M4 launcher-ui (the launcher's own
  `launcher_namespaces` interleaved into the same `collect_stats`, F169). Every conflict outside
  `FINDINGS.md` (itself append-only on both sides, resolved by concatenating and — only where a
  number collided — renumbering the newer side) was a small, mechanical interleave: two
  functions or two `.chain()` calls sitting next to each other, never a real disagreement about
  what either wave's code should do. Separately, the shared nested cosmic-comp
  (`dev/nested.sh`'s `wayland-1`) leaked to 64 GB resident on the NVIDIA EGL path mid-wave and
  was killed and restarted on Mesa by the coordinator; every `sill run` this wave had against the
  old instance died with it. `dev/accept-wallpaper.sh` does not itself start or stop the
  compositor (`dev/nested.sh start` is a separate, manual step, per its own header) — only the
  `sill run` it starts, and it never touches `NESTED_COMPOSITOR_PID`.
- **F171 The wallpaper's own image never paints on the nested compositor: the surface exists,
  is correctly positioned and sized, and shows its own material fallback — the loaded image
  itself is the one thing that never appears.** Investigated thoroughly, on the running nested
  cosmic-comp (Mesa), not through any injected variable or forced backend
  (CONVENTIONS "run it the way its user would"):
  - The cache pipeline is correct end to end: `~/.cache/sill/wallpaper/<hash>-<w>x<h>.png` is a
    real 2176 x 1360 PNG whose centre pixel is exactly the source's marked colour, checked with
    PIL directly against the file on disk, independent of anything sill or shell-host render.
  - The `file://` URL sill builds matches shell-host's own documented shape exactly
    (`dom/net.rs`'s module doc: `background-image: url(file:///…)`); a `sill debug capture` of
    the live surface is uniform black (or, once the compositor's real dark `--paper`/Widget
    fallback shows through cleanly, that fallback colour) — never the marked colour — however
    long the daemon has been running (checked immediately after start and again minutes later).
  - Switching from a CSS `background-image` on an empty `div` to a plain `<img src="...">`
    (this wave's own fix, believing the div's declarative background load was the gap) changed
    nothing: still black, still never settles to the image.
  - **The `file://` scheme is not the variable.** The same `<img>` given a self-contained
    `data:image/png;base64,...` URL (a 4 x 4 red PNG, no network provider round trip at all)
    is *also* never painted — ruling out `LocalNet`'s file read entirely. No image source this
    wave tried, through either markup shape, ever reaches the screen.
  - `sill debug stats`' frame counter is the other half of the picture: queried repeatedly back
    to back (no `sleep` between calls) it never moves on its own; queried once shortly after
    `sill run` starts and again 5 s later it is often unchanged (frames stay at 1, the initial
    commit) — but `dev/accept-wallpaper.sh`'s own run, which checks readiness and idles within
    a second or two of startup, sees a burst instead (2 → 69-139 across runs) that always
    settles by the time a later, unhurried check is made. Read together: the surface's *first*
    commit happens unconditionally: everything after it — the image resource resolving, and
    any repaint that would show it — depends on the surface receiving further paint
    opportunities, which arrive in a bounded burst right after creation and then stop for good.
  - **Working theory, not confirmed inside this repo:** a `Layer::Background` surface with
    `ExclusiveZone::Ignore` and `RegionSpec::Empty` input (this one) is the kind of layer-shell
    client cosmic-comp (or shell-host's render-loop gating on a `wl_surface.frame` callback,
    F21 in shell-host's own FINDINGS) is least motivated to keep scheduling frames for — nothing
    ever interacts with it, nothing overlaps it, and most real Background clients (a wallpaper
    daemon) paint once and never again, so this combination may simply be under-exercised. The
    bar (`Layer::Top`, real input) keeps receiving frames indefinitely (FINDINGS F80: "8 frames
    and 8 commits... before and after five idle seconds" — moving, not frozen); this surface is
    the first `Layer::Background` client either shell-host or cosmic-comp has had to carry a
    resource that loads *after* its first commit.
  - Not this wave's to fix: `shell-host` (the render-loop/frame-callback gating) and possibly
    `cosmic-comp` itself are both outside this wave's ownership and outside this repo. Reported
    rather than patched (CONVENTIONS §1: "If you believe a frozen signature is wrong, stop and
    report it").

### Gaps found in M2 wallpaper (continued)

- **G21** A `Layer::Background` surface whose content depends on a resource that resolves after
  the surface's first commit (an async image load, here) never gets the second paint it needs:
  observed as a frame counter frozen at 1 well after the image is cached and ready, on a real
  nested cosmic-comp, independent of the URL scheme (`file://` or a self-contained `data:` URI)
  or the markup (`background-image` or `<img src>`) carrying it (F171). Whether the gap is in
  shell-host's frame-callback-gated render loop, in cosmic-comp's scheduling for a
  Background-layer, non-interactive client, or both, is not established from inside this repo;
  reported for the shell-host/host team to chase with a minimal reproduction (a `Layer::Background`
  surface that mutates its own DOM shortly after its first frame, independent of any image).

## Wallpaper merged behind `spaces.wallpaper_drawer`

- m2-wallpaper merged with accept-wallpaper at 2/4 (centre pixel and idle fail on the host's
  F171/G21). Until shell-host repaints a background layer after its first commit, the daemon
  spawns the wallpaper only when `spaces.wallpaper_drawer = "shell"` (default `cosmic`, so
  `cosmic-bg` keeps drawing and the bar/dock/launcher acceptance over `dev/backdrop.sh` is
  unchanged). `dev/accept-wallpaper.sh` sets the key for its run and restores the settings file.
- design/22 row for quire (§3.14 Spaces, Advanced): `spaces.wallpaper_drawer` enum
  `Cosmic | Shell`, default `Cosmic`, "COSMIC's own background service, or the shell's wallpaper,
  which cross-fades with light and dark." Listed in `tests/keys.rs` `AWAITING_ROWS` until then.
- When G21 is fixed: flip the default to `Shell`, re-run accept-wallpaper and accept-{bar,dock}.

## Displays (2026-09-25)

The `displays` service family in `sill-services` (`src/displays/`), the `display.*` settings
domain, `sill display {brightness,volume} ...`, and the brightness and volume keys in
`dist/cosmic`. Everything is derived from what a display reports (EDID, DDC/CI, the compositor's
heads); nothing is keyed to a vendor or a model, and every default has a `display.*` key.

### Scale

- **Policy** (`displays::scale`, table-tested): density along the diagonal from the EDID's
  preferred-timing millimetres (else the base block's centimetres, else the compositor's
  `physical`), divided by `display.density_target_ppi` (109, Apple's 27-inch 2560 x 1440), then
  rounded per `display.scale_rounding`. With no usable size (0 mm, or a density outside 40-600
  ppi such as a TV reporting 16 x 9 cm): 2 when the panel is at least 3840 px wide, else 1 -
  `scale_rounding` plays no part there, since there is no density to round.
  - **`Nearest`**: the integer when within 0.15 of one, else the nearest of 1, 1.25, 1.5, 1.75,
    2, 2.5, 3.
  - **`PreferWhole`** (the default, added 2026-09-25): macOS's own default for a sharp external
    display - a 4K screen defaults to 2x there, "looks like 1920x1080" - rather than the
    fractional scale that gives more logical space but a softer render. Below 1.5, `Nearest`'s
    answer; at or above it, the ideal rounded half-up to a whole scale (1, 2 or 3) so long as
    that leaves at least `MIN_LOGICAL_WIDTH` (1280) logical pixels across, else `Nearest`'s
    answer. The user tried both on their 27" 4K (BenQ MA270U) and preferred 2.0 over 1.5.

  | display | ppi | raw | Nearest | PreferWhole |
  |---|---|---|---|---|
  | 27" 4K (and the MA270U's own 597 x 336 mm) | 163 | 1.50 | 1.5 | 2 |
  | 27" 5K | 218 | 2.00 | 2 | 2 |
  | 24" 1080p | 92 | 0.84 | 1 | 1 |
  | 13.3" 2560 x 1600 | 227 | 2.08 | 2 | 2 |
  | 14" 2880 x 1800 | 242 | 2.23 | 2 (outside the integer pull, but 2 is still the nearest step) | 2 |
  | 32" 4K | 138 | 1.26 | 1.25 | 1.25 (below 1.5, unchanged) |
  | 27" 1440p | 109 | 1.00 | 1 | 1 |
  | 15.6" 1080p | 141 | 1.29 | 1.25 | 1.25 (below 1.5, unchanged; Windows' own 125% for these) |
  | 14" 1920 x 1200 | 162 | 1.48 | 1.5 | 1.5 (below 1.5, unchanged) |
  | 16" 2560 x 1600 | 189 | 1.73 | 1.75 | 2 |
  | 13.6" 2560 x 1664 | 224 | 2.06 | 2 | 2 |
  | 34" 3440 x 1440 | 110 | 1.01 | 1 | 1 |
  | 32" 6K | 216 | 1.98 | 2 | 2 |
  | 13.5" 2256 x 1504 | 201 | 1.84 | 1.75 | 1.75 (whole scale 2 wants 2256 / 2 = 1128 px, under `MIN_LOGICAL_WIDTH`, so this falls back) |
  | 15.6" 4K | 283 | 2.59 | 2.5 | 3 (3840 / 3 = 1280, exactly `MIN_LOGICAL_WIDTH`) |
  | a projector at 1080p / 4K (0 mm) | - | - | 1 / 2 | 1 / 2 |

  No common laptop came out absurd, so the rule stands as specified; the one addition is the
  plausibility window. For comparison, cosmic-comp 1.8 picks its own default for a new output
  (read for the behaviour only; it is GPL): 4K always 2, 50 % steps, different step sizes for
  laptops and desktops, and a cap by the shorter side. On the MA270U COSMIC gives 2 and sill,
  under its new default, now agrees.
- **Applying** (`displays::seen`, `scale_pass`, `sink`): only on the first sight of an identity
  (`IdentityKey`: `BNQ-8074-<FNV-1a of make, product and serial>`; the serial is never stored
  in clear), recorded in `$XDG_STATE_HOME/sill/displays.json`. A recorded display is never
  rescaled, and `display.scale = "manual"` records without setting. Since the scale pass only
  ever runs on a display's first sight, `display.scale_rounding`'s new default needs no
  migration: it changes nothing for a display sill has already recorded, and reaches only a
  display connected from here on. `display.scale_overrides` is the one way sill changes a known
  display: a `<identity> = <scale>` pair applies when it is
  added or edited (it is compared with the scale sill last set, so a later change in COSMIC
  Settings stands). A failed set records nothing, so the next output change retries.
- **Route**: cosmic-comp 1.8 persists output state itself in
  `$XDG_STATE_HOME/cosmic-comp/outputs.ron`, keyed by the set of connected
  `(connector, make, model)` and holding mode, scale, transform and position; it applies and
  persists changes made through `zwlr_output_manager_v1` (v4). sill goes through the
  `cosmic-randr` tool COSMIC ships (`cosmic-randr list --kdl`, then `cosmic-randr mode <name> <w>
  <h> --refresh <hz> --scale <s>`) behind the `ScaleSink` trait (a fake in the tests). Under KWin
  `cosmic-randr` finds no output manager and the pass changes nothing, so running sill in the
  KDE session cannot touch KDE's 1.7.
- **Live, nested** (a private `cosmic-comp --no-xwayland` with `COSMIC_BACKEND=winit` and its
  own `XDG_STATE_HOME`, so `~/.local/state/cosmic-comp` was never written): the one head is
  `WINIT-0`, make `COSMIC`, model `WINIT-0`, `physical 0 0`, 2176 x 1360 at 60.000 Hz, scale
  1.00, transform flipped180. With no EDID and no size its default is 1, so the daemon's first
  pass recorded `COSMIC-0000-8eb9a3b360803609` with nothing applied. An override row set 1.25
  through the running daemon (cosmic-comp reported `scale 1.25` and wrote it to its
  outputs.ron); removing the row left 1.25 alone, and setting 1.00 by hand afterwards was not
  fought. `tests/scale_live.rs` (ignored; it refuses unless every head is `WINIT-*`) sets and
  restores a scale through `CosmicRandr` and passes.

### EDID and gamut

- `displays::edid` is a hand-written base-block parser (manufacturer, product, serial string or
  number, 0xFC name, the preferred timing and its millimetres, primaries and white): the block
  is a fixed layout of a few dozen bytes, and the crates on offer either bring a
  parser-combinator stack or bind libdisplay-info through C. The MA270U's own EDID (serial string
  masked) is the fixture: R .681/.316, G .256/.680, B .152/.062, W .313/.329, 597 x 336 mm,
  3840 x 2160 at 59.996 Hz.
- `displays::gamut::classify`: Display P3 when the panel's triangle covers at least 90 % of
  P3's and is at most 1.15 x P3's area; Wide when it is otherwise at least 1.2 x sRGB's area
  (Adobe RGB, Rec.2020); else sRGB. The MA270U is P3; a typical sRGB panel, sRGB itself and a
  72 % NTSC laptop panel are sRGB; Adobe RGB and Rec.2020 are Wide.

### DDC/CI

- **Crates**: `ddc` 0.2 (the VCP trait) and `ddc-i2c` 0.2.2 with `with-linux` (not
  `with-linux-enumerate`, which links libudev); both MIT, as are `i2c`, `i2c-linux`,
  `i2c-linux-sys`, `resize-slice` and `uninitialized` under them; `cargo deny check licenses`
  passes. `ddcutil` is not needed at run time. They sit after the pinned block in
  `[workspace.dependencies]` (sill-only, not part of quire's `docs/workspace-deps.toml`).
- **Finding the monitors**: NVIDIA's driver has no `card*-<connector>/ddc` link, so the join is by
  EDID: every `/dev/i2c-N` whose adapter hangs off a PCI display controller (class `0x03xxxx`) is
  asked for its EDID at 0x50, the connector with the same base block is its monitor, and a bus
  that also answers a VCP read is a DDC display. On this machine: bus 5 = `HDMI-A-2` = BenQ
  MA270U.
- **Measured on the MA270U** (`tests/ddc_live.rs`, ignored; every value is restored and the
  restore asserted): a probe (EDID reads on every display bus plus the first VCP read) takes
  158-182 ms; the first read 50-53 ms; each further read about 100 ms (the crate's 50 ms delay
  between commands plus the transaction), 109-112 ms averaged over ten back-to-back reads; a raw
  write 54-57 ms; a verified write (write, then read back) 155-160 ms for brightness and about
  352 ms for speaker volume, whose read-back still shows the old value the first time, so it
  takes a second round. Through the service a slider's value reaches the monitor 120 ms (the
  debounce) after it stops moving, plus one verified write.
- **The bus is lossy**: about one transaction in 25 came back "expected DDC/CI length bit" (seen
  on 0x1A and 0x62), and one write that followed such a failed read returned Ok and was dropped
  by the monitor (the protocol has no acknowledgement; that run left the volume at 18 until it
  was put back by hand to 20). So `ddc::retry` retries a read three times and verifies every
  write by reading it back, repeating it up to three times, as ddcutil does by default.
- **Service shape**: one worker thread owns every bus (`ddc::worker`); writes are debounced per
  (display, feature), latest wins, 120 ms after the last change (`ddc::debounce`, pure); the
  published `DdcDisplay`s are the read cache (read at probe, then following sill's own writes);
  a failed read makes that control absent and is logged once per bus, feature and direction.
  The buses are re-probed when the set of outputs changes, not on every mode or scale change.
- **Capabilities** (`ddc::Controls`, MA270U): brightness 49/100, contrast 50/100, colour preset
  0x0B (user 1), RGB gains 100/100 each, input 0x11 (HDMI 1; read, never written live), speaker
  volume 20 out of **50** (not 100), mute 0x02 (audible).

### Brightness, volume, warmth, fonts

- **Routing** (`brightness::route`): eDP, LVDS and DSI connectors go to a backlight through
  logind's `Session.SetBrightness("backlight", name, value)` on
  `/org/freedesktop/login1/session/auto` (firmware over platform over raw when there are several;
  never below the lowest non-zero value); other connectors go to the DDC display on that
  connector; anything else to nothing. Keys move along a 16-step ladder, as on a Mac keyboard; a
  slider sets the level outright.
- **Which output**: `display.brightness_keys_target = "pointer_output"` acts on the output of
  the active window, since the pointer's output is only the compositor's to know (F140);
  `all_outputs` acts on every one. There is no OSD surface yet; `Displays::brightness(target)`
  is what it will read.
- **Keys**: `dist/cosmic/.../system_actions` binds COSMIC's `BrightnessUp`/`BrightnessDown`,
  `VolumeRaise`/`VolumeLower` and `Mute` (XF86MonBrightness*, XF86Audio*) to `sill display ...`.
  While installed, this replaces cosmic-settings-daemon's own brightness handling and
  cosmic-osd's popup for those keys.
- **Monitor speakers** (`hw_volume`): pipewire-pulse names an HDMI or DP sink after the monitor's
  ELD, `... Digital Stereo (HDMI) [BenQ MA270U]`. When that bracketed name is a DDC display's EDID
  name and the display has VCP 0x62, a change the person makes to the software level or mute
  (never a sink switch, never sill's start) is sent on as the same percentage of the monitor's
  range, and as VCP 0x8D. The software path is unchanged, so the two levels multiply: the
  conservative choice, since turning the software attenuation off would surprise anyone who uses
  the sink elsewhere. The default sink here is a Bluetooth headset, so this was checked with the
  table tests and the MA270U's real sink name, not live.
- **Night warmth** (`warmth`, `plan::warmth_writes`): the strength maps to 6500-2700 K linearly
  in mireds, then to per-channel multipliers from the public-domain blackbody fit, normalised so
  that 6500 K is exactly neutral; each display's own gains are remembered the first time and
  restored when it is turned off. Off by default. The originals live in memory only, so a crash
  while warm leaves the monitor warm until its preset is chosen again.
- **Font rendering** (`font`, `fontconfig_file`): greyscale antialiasing, `rgba none`, and
  `hintslight` below 150 ppi or `hintnone` from 150, following the densest output. Written to
  `$XDG_CONFIG_HOME/fontconfig/conf.d/50-sill-rendering.conf` with a header that marks it as
  sill's, only under `display.font_rendering = "auto"`, and only when no other conf.d file (or
  `fonts.conf`) has an `<edit name=...>` for antialias, hinting, hintstyle or rgba; removed when
  the setting is `off` or such a file appears; a same-named file without the header is never
  touched. Tested in a temporary directory only: nothing was written under this machine's home.
  In the nested session there is no density (WINIT-0 reports 0 mm), so nothing is written.

### Colour management (investigation only)

- **cosmic-comp 1.8.0** (source at `epoch-1.8.0`, and `wayland-info` on the nested compositor):
  no `wp_color_manager_v1`, no `wp_color_representation_manager_v1`, no
  `frog_color_management_factory_v1`, and no `zwlr_gamma_control_manager_v1` either (so no
  gammastep or wlsunset). The KMS backend sets `max bpc` per connector and nothing else colour
  related: no GAMMA_LUT, DEGAMMA_LUT or CTM property is written, so there is no output colour
  transform of any kind, and a P3 panel shows sRGB content oversaturated. The output globals it
  does offer: `zwlr_output_manager_v1` v4, `zcosmic_output_manager_v1` v3, `wl_output` v4,
  `wp_fractional_scale_manager_v1` v1, `zxdg_output_manager_v1` v3.
- **KWin 6.7.5** (the host session): `wp_color_manager_v1` **v2** (intents perceptual, relative,
  absolute, relative_bpc; features parametric, extended_target_volume,
  set_mastering_display_primaries, set_primaries, set_luminances, windows_scrgb; transfer
  functions gamma22, st2084_pq, ext_linear, bt1886; primaries srgb, pal_m, pal, ntsc,
  generic_film, bt2020, cie1931_xyz, dci_p3, display_p3, adobe_rgb), plus
  `frog_color_management_factory_v1` v1 and `wp_color_representation_manager_v1` v1.
- So `display.color_management` (`auto` | `off`) is stored and does nothing yet; the gamut
  `classify` computes is what it will hand to the compositor once cosmic-comp offers
  `wp_color_manager_v1`.

### ddcutil permissions

- Fedora 44's `ddcutil` 2.2.1 installs `/usr/lib/udev/rules.d/60-ddcutil-i2c.rules`
  (`SUBSYSTEM=="i2c-dev", KERNEL=="i2c-[0-9]*", ATTRS{class}=="0x030000", TAG+="uaccess"`) and
  `/usr/lib/modules-load.d/ddcutil.conf` (`i2c-dev`). With it installed, the user at the active
  seat gets an ACL on every VGA-class adapter's bus (`getfacl /dev/i2c-5`:
  `user:pohsuanlai:rw-`), in the `i2c` group or not; that group here comes from a hand-written
  `/etc/udev/rules.d/99-i2c-dev.rules` and is not needed. Without ddcutil's package, and outside
  such a group, `/dev/i2c-*` is root's (or does not exist without `i2c-dev`), every probe finds
  nothing, and the brightness keys on an external monitor quietly do nothing.
- So sill ships `dist/udev/60-sill-ddc.rules` (the same `uaccess` tag for any display-controller
  class `0x03????`, which also covers the `0x0302xx` discrete GPU of hybrid laptops that
  ddcutil's rule misses) and `dist/modules-load.d/sill-ddc.conf`, for packagers on distributions
  without ddcutil's rule. `uaccess` reaches only the active local session: a remote login or a
  background session gets nothing. Checked with `udevadm verify`.

### Gaps and choices

- **quire gap**: the settings derive's `Vec<_>` shape needs an enum element, and there is no list
  of text, so `display.scale_overrides` is one text key of `;`-separated pairs. A `List(Text)`
  kind would let it be a rows editor.
- **design/22 rows to add** (§3, a new `display` section; all Advanced until there is a Displays
  page), listed in `tests/keys.rs` `AWAITING_ROWS` until they land:

  | Key | Type | Default | Meaning |
  |---|---|---|---|
  | `display.scale` | `Auto \| Manual` | `Auto` | On first sight of a display, set the scale its density calls for; Manual never changes a scale. |
  | `display.scale_overrides` | text | `""` | `<identity> = <scale>` pairs, `;`-separated; each applied when added or edited. |
  | `display.density_target_ppi` | count 72..=220 ppi | 109 | The logical density the automatic scale aims for. |
  | `display.brightness_keys_target` | `PointerOutput \| AllOutputs` | `PointerOutput` | Which displays the brightness keys and slider act on. |
  | `display.hardware_volume` | `Off \| WhenMonitorIsOutput` | `WhenMonitorIsOutput` | A monitor's own speakers follow volume changes over DDC/CI. |
  | `display.night_warmth` | `Off \| Hardware` | `Off` | Warm external monitors through their RGB gains. |
  | `display.night_warmth_strength` | percent 0..=100 | 50 | 0 is 6500 K, 100 about 2700 K. |
  | `display.font_rendering` | `Auto \| Off` | `Auto` | sill's fontconfig snippet for other apps (greyscale antialiasing, hinting below 150 ppi). |
  | `display.color_management` | `Auto \| Off` | `Auto` | Use each display's reported gamut once the compositor supports colour management; stored only for now. |
- **IPC**: `Request::Display(DisplayRequest::{Brightness(LevelRequest), Volume(VolumeRequest)})`
  is a new variant; its wire spelling is pinned in `sill-ipc/tests/serde.rs`.
- **Not done**: an OSD and a control-center slider (surfaces; `DisplaysCommand::{Brightness,
  SetFeature}` and `Displays` are their API); a schedule for night warmth; hardware volume
  checked live (the default sink here is Bluetooth); the pointer's own output (approximated by
  the active window's).

## Wallpaper paints (2026-09-25)

- **F172 The wallpaper's whole document was 0 px tall, so the surface painted nothing; with
  G21 gone (shell-host F42), that was the rest of F171.** Reproduced offline in
  `tests/wallpaper_paint.rs` (a `ds_native::Harness` painted over `Backdrop::Clear`): every
  rect from `html` to the image layers was 320 x 0, and the centre pixel was (0,0,0,0) with an
  image and without one. Two causes, one after the other: `.sill-wallpaper{height:100%}`
  resolved against `.ds` and `#main`, neither of which has a height (shell-host F41); and the
  WIP's `position:fixed; inset:0` did not help, since Taffy places a fixed box against its
  parent (the 0 px `.ds`), not the viewport. `.sill-wallpaper` is now `width:100vw;
  height:100vh; position:relative`, as `.sill-launcher-catcher` already was, and the two tests
  (the fixture PNG's colour at the centre and a corner; an opaque `--paper` with no image) pass.
- The live idle "failure" (3 → 70, then 3 → 9 frames) was the acceptance measuring too early:
  sampled every 0.5 s, the surface goes 2, 3, 4, 9 frames over the ~2 s it takes the debug
  build to resize the source and cross-fade it in over `--paper`, then stays at 9 for 20 s.
  `step_centre_pixel` now polls the capture for up to 10 s until the image has landed, so the
  idle window measures a settled surface. accept-wallpaper: 4/4.

## Icon style (2026-09-25)

The dock and the launcher draw app icons in quire's icon style (quire CONSUMING "App icons and
the icon style"; `crates/sill-surfaces/src/app_icons/`).

- **Shipped icons.** `dock.shipped_icons = on` (default; both the dock and the launcher follow
  it) gives an entry quire's own icon when its desktop id is in `app_icons::DEFAULT_TABLE`,
  compared without regard to case, whole id only: COSMIC Terminal, Files and Edit; ghostty,
  foot, kitty, Alacritty, GNOME Terminal and Console, Konsole (terminal); Nautilus, Dolphin,
  Thunar (files); mailo, Thunderbird, Geary, Evolution (mail); GNOME Text Editor, Kate (notes);
  Loupe, Eye of GNOME, Gwenview, Shotwell (photos). No category rule: `App` keeps no
  `Categories=`, and a wrong match is worse than a theme icon. The shipped file carries its
  own plate, bevel and shadow at the tile's full size, so the dock draws it bare over the
  whole tile (`.sill-dock-art`), not inside quire's plate.
- **Sizes.** The raster is the smallest of quire's 13 sizes covering the drawn size times the
  output scale (`app_icons::pick`: 48 at 1x, 1.5x, 2x, 1.75x is 48, 72, 96, 96). A dock tile is
  loaded for the largest size it is drawn at (the magnified size when magnification is on),
  so a magnification sweep never swaps URLs (each swap would load a file and blank the icon
  for a frame); the rows use 48. `pick` stays sill's own, in front of `ds_settings::app_icon_path`
  (Q71, resolved below): it decides which of the 13 sizes to ask for, which the retint cache key
  needs regardless (and which is what keeps a magnification sweep on one file, not a new one
  every frame) — a call `app_icon_path` cannot make for sill, so it is behaviour quire does not
  cover, kept and documented in `app_icons::size`.
- **Re-tinting.** Muted and Monochrome put every third-party raster, and our Monochrome files,
  through `ds::icon::retint` once per key (icon, size at scale, style, tint; the tint rounded
  to 0.1 degree and 0.0001 chroma, and neutral under Colour and Muted so a Space switch
  re-keys only Monochrome). Theme SVGs are rasterised with resvg (0.48, MIT OR Apache-2.0, the
  renderer of the usvg and tiny-skia already in the tree). The work runs on tokio's blocking
  pool; the first render shows the untinted file and every `use_icon_resolver` caller
  re-renders when a raster lands. The Monochrome tint is the active Space's
  (`Tint::space(&look.dots)`), the card accent (`Tint::from_hex`: the Space's accent under
  `CardAccent::SpaceHue`, else `quad(appearance.accent, Light)`), or neutral.
- **Cache as files**, `$XDG_CACHE_HOME/sill/icons/<stem>-<px>-<style>-<fnv64>.png`, written
  through a temporary file and a rename, and remade when older than its source. An `IconUrl`
  is `file:` or `data:`; a `data:` URL puts ~20 KB of base64 into a tile's inline style, which
  the dock rewrites every magnification frame, and it would be re-tinted on every restart.
- **The plate is re-tinted too** (Q72, resolved below). Wherever a third-party icon sits on
  quire's neutral plate (the dock tile; the launcher rows draw no plate — `ds::Tile::Source` is
  "drawn as it is, with no plate of its own"), `IconView` takes the same `(IconStyle, Tint)`
  pair the icon's own raster is handed to `retint`, as `plate_tint: PlateTint::of(style, tint)`;
  `None` under Colour changes nothing. quire re-tints the family's two stops and its glyph ink
  by the same OKLCh rule as a raster's pixel, for both schemes at once, and writes them as the
  plate's own custom properties (`--plate-base-l` … `--plate-ink-d`), read under
  `data-icon-style` and the root's `data-theme`; no selector on `.ds-plate`, no hex colours
  inline, so the markup lint stays clean with no exception. quire's one stated limit: in the
  light scheme the neutral plate's top-left stop is white, and `retint` eases chroma to nothing
  at white, so it stays white under a tint; the tint shows at the deep (bottom-right) stop and
  the glyph ink instead. The dark plate (`#2A2E28` before) takes the tint at both stops.
- **Seen live** (`dev/accept-icons.sh`, nested cosmic-comp, scratch config and cache, 7/7):
  Terminal and Files from quire's files, Chrome and btop on the plate; Muted writes 2 rasters,
  Monochrome 4 (the two theme icons and our two Monochrome files), in the Work preset's
  blue-violet; 0 frames over 5 idle seconds after they land. `dev/out/dock-icons-colour.png`,
  `dock-icons-muted.png`, `dock-icons-monochrome.png`. btop's SVG fills with `<pattern>`s,
  which Blitz paints black and resvg red, so its re-tinted rasters are closer to the icon as
  drawn than its Colour tile. Under Monochrome, Chrome's and btop's plates are visibly the
  Work preset's blue-violet, against white under Colour and Muted (Q72): a last switch to
  `appearance.theme = "dark"` in Monochrome (`dock-icons-monochrome-dark.png`) shows the dark
  plate taking the tint at both stops, richer than the light shot, where the top-left stop
  stays near white and only the deep stop and the ink carry it.

### Gaps

- **Q71 (quire), resolved (2026-09-25).** sill no longer searches for the app icon files itself:
  `$SILL_ICON_ASSETS` and sill's own XDG-then-sibling-checkout search are gone.
  `ds_settings::apps_dir()` and `ds_settings::app_icon_path(app, Px(tile_px * scale), IconStyle)`
  (the exact size, else the nearest larger, else the largest smaller) are quire's own lookup and
  size match (quire CONSUMING "App icons and the icon style"). Both touch the disk on every
  call, so `app_icons::cache::IconResolver::shipped_path` caches the resolved path (or its
  absence) per `(app, px, style)`, in the same process-wide cache the re-tinted raster itself
  lives in, resolved once and never looked up again for that key. `app_icons::size::pick` stays
  in front of it (behaviour quire does not cover): it is what decides the size to ask for at
  all — the retint cache key needs it, and it is what keeps a dock tile's magnification sweep
  on one file rather than a new one every frame — and quire ships every one of the 13 sizes for
  every app and style, so the size `pick` names is always an exact match.
- **Q72 (quire), resolved (2026-09-25).** `IconView` takes `plate_tint: Option<PlateTint>`
  (`PlateTint::of(style, tint)`, `None` under Colour); quire re-tints the plate's two stops and
  its glyph ink itself, by the same rule as a raster's pixel, and writes them as the plate's own
  custom properties under `data-icon-style`, so sill never touches `.ds-plate` or writes a hex
  colour inline. quire's stated limit: in the light scheme the top-left stop is white and
  `retint` eases chroma to nothing there, so it stays white under a tint; the tint shows at the
  deep stop and the ink. `dock/tile.rs`'s `IconView { plate: Some(PlateFamily::Neutral),
  plate_tint: PlateTint::of(look.0.style, look.0.tint), .. }` is the one call site: the launcher
  rows draw no plate at all (`ds::Tile::Source`'s own doc: "with no plate of its own under it").
- **Q73 (design/22)** `dock.shipped_icons` has no §3.5 row; `AWAITING_ROWS` exempts it:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `dock.shipped_icons` | `ShippedIcons::{On,Off}` | `On` |  | `08-ICONS.md` 2.11; sill `FINDINGS.md` "Icon style" | proposed (2026-09-25) |

  Advanced for now (§5 lists the Dock page's Basic keys); it also governs the launcher's rows.

## OSD (2026-09-25)

design/20 §1.7: one Overlay layer per output (`sill-osd-<output>`), spawned hidden and kept warm
with `set_visibility` like the launcher; anchored to the `osd.position` edge, centred, exclusive
zone `Auto` (reserves nothing and sits clear of the dock's reserve without reading its geometry),
keyboard `None`, input `Empty`, blur `Element("osd")`. A 280 x 52 card (design leaves the size
open) with 40 px of transparent room for the Osd material's ambient shadow.

- **Trigger policy.** The OSD shows only the changes sill makes: `sill display volume ...` and
  `sill display brightness ...` (what the keys run), after the service call succeeds, plus
  `sill debug osd volume|brightness N [--mute muted]`, which shows a made-up level and moves
  nothing. A change from another app (a player's slider, `pactl`, the monitor's buttons) never
  shows: macOS shows its indicator for the keys only, and following every PipeWire change would
  flash it whenever an app ramps its stream. `osd.enabled = "off"` silences all of it.
- **Where.** Volume: the active window's output (the stand-in for the output under the pointer,
  F140), else the first. Brightness: the outputs `display.brightness_keys_target` picks, chosen
  with the service's own `targets`, minus those with no brightness to show.
- **What.** The glyph at `IconSize::Bar` (`volume-x` muted or 0, then `volume`, `volume-1`,
  `volume-2` by thirds; `sun` for brightness) and quire's `Slider` read-only (`Disabled`, empty
  when muted), on a painted Osd root tinted by the active Space, inside a transparent Osd root
  whose box fades: in with `fade` over `--t-quick --e-out`, out with `menu-out` over
  `--t-move --e-exit`. The monitor's own speakers follow the sink, so volume is one level.
- **Timing.** `osd/machine.rs` is pure (table-tested): a change maps it and holds
  `osd.hold_ms` from the last change; a change during the fade takes it back and holds again;
  the fade settles at `DurationToken::Move + FRAME_SLACK`, then it unmaps and no timer runs. The
  surface wakes it with `ds::sleep` only.
- **Seen live** (`dev/accept-osd.sh`, nested cosmic-comp, 4/4): hidden at start; shown 83 ms
  after `sill debug osd volume 60`; hidden by 1828 ms (hold 1500); a change 100 ms into the fade
  keeps it up at 2480 ms; 0 frames in 5 idle seconds after the hide. `dev/out/osd-cosmic.png`.

### Gaps

- **Q74 (quire)** `Slider` has no read-only level form: design/20 §1.7's level bar is "`Slider`
  read-only", but the Slider always draws its thumb and is focusable, and its fill does not
  transition, so the "level change `--t-quick --e-out`" cannot be drawn without styling
  `.ds-slider-fill`. sill passes `Availability::Disabled` and the bar jumps between levels. A
  `SliderMode::Level` (no thumb, no tabindex, fill width transitioning `--t-quick --e-out`)
  would close it.
- **Q75 (quire)** No `Anim` for the OSD's exit (`fade`/`menu-out` at `--t-move --e-exit`): sill
  writes the animation in its own sheet and computes the settle from
  `DurationToken::Move + FRAME_SLACK` rather than `ds::settle`. An `Anim::OsdIn`/`OsdOut` pair
  (or an `Osd` component owning the card and its fade) would make the timer quire's.
- **Q76 (quire)** Fading a painted root as a whole needs the motion tokens outside it: sill
  nests a painted Osd `Ds` (`stylesheet: Inject::Host`) inside a transparent Osd `Ds` so the
  fading box can read `--t-*`. An `Osd` component in quire would own that.
- **Q77 (design/22)** No `osd` rows; `AWAITING_ROWS` exempts them (a new §3.16 `osd`):

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `osd.enabled` | `OsdEnabled::{On,Off}` | `On` | shows sill's own volume and brightness changes | `20-SURFACES.md#1-7-osd`; sill `FINDINGS.md` "OSD" | proposed (2026-09-25) |
  | `osd.hold_ms` | `Ms` | `1500` | `300..=10000`; after the last change, before the fade | `20-SURFACES.md#1-7-osd` | proposed (2026-09-25) |
  | `osd.position` | `OsdPosition::{Bottom,Top}` | `Bottom` | centred along that edge | `20-SURFACES.md#1-7-osd` | proposed (2026-09-25) |
  | `osd.bottom_margin_px` | `Px` | `24` | `0..=400`; gap from the dock's reserve (or the bar's, at the top) to the card | sill `FINDINGS.md` "OSD" | proposed (2026-09-25) |

  All Advanced until §5 places them on a page.
