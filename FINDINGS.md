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
keyboard `None`, input `Empty`, blur `Element("osd")`. A 296 x 80 card (design leaves the size
open; 296 is quire's `Osd` card width, a control-center module spanning both columns) with 40 px
of transparent room for the Osd material's ambient shadow.

- **Trigger policy.** The OSD shows only the changes sill makes: `sill display volume ...` and
  `sill display brightness ...` (what the keys run), after the service call succeeds, plus
  `sill debug osd volume|brightness N [--mute muted]`, which shows a made-up level and moves
  nothing. A change from another app (a player's slider, `pactl`, the monitor's buttons) never
  shows: macOS shows its indicator for the keys only, and following every PipeWire change would
  flash it whenever an app ramps its stream. `osd.enabled = "off"` silences all of it.
- **Where.** Volume: the active window's output (the stand-in for the output under the pointer,
  F140), else the first. Brightness: the outputs `display.brightness_keys_target` picks, chosen
  with the service's own `targets`, minus those with no brightness to show.
- **What.** quire's `Osd` card (CONSUMING.md "OSD parts", 2026-09-25 on): a title line over a
  read-only `LevelControl` in `LevelLook::Capsule` (sill's `view.rs::LOOK`, the one place to
  change it once the user picks a look from quire's sheets), tinted by the active Space, inside
  the one transparent Osd root that gives it every token (no nested painted root: Q76, closed).
  The title is the real name where the services give one — the volume service's sink
  description, the displays service's monitor name (its DDC product, when the output's
  brightness target is a DDC display) — else "Sound"/"Display" (`reading::title`). The glyph is
  `LevelGlyph::Volume(Muting::{Audible,Muted})` (no sink counts as muted) or `::Brightness`;
  `LevelControl` draws the waves or the sun itself from the fill and the mute (Q74, closed).
  The monitor's own speakers follow the sink, so volume is one level.
- **Timing.** `osd/machine.rs` is pure (table-tested) and now only a show-and-hold: a change
  shows it (`Shown::Visible`) and holds `osd.hold_ms` from the last change; the hold's end hides
  it (`Shown::Hidden`). Quire's `Osd` card owns the entrance and exit fade from that `shown`
  (`Anim::OsdIn`/`OsdOut`, Q75, closed) and calls `on_hidden` once the exit has settled
  (`ds::settle`, not sill's own `DurationToken::Move + FRAME_SLACK`); `drive.rs` unmaps the
  surface there, not when the hold ends, so a show during the fade cancels it without a second
  entrance. The surface wakes the hold machine with `ds::sleep` only.
- **Seen live** (`dev/accept-osd.sh`, nested cosmic-comp, 4/4, against quire's `Osd` card):
  hidden at start; shown 114 ms after `sill debug osd volume 60`; hidden by 1842 ms (hold 1500);
  a change 100 ms into the fade keeps it up at 2557 ms; 0 frames in 5 idle seconds after the
  hide. `dev/out/osd-cosmic.png` (real sink title "USB Audio S/PDIF Output") and, muted
  (`sill debug osd volume 60 --mute muted`), `dev/out/osd-cosmic-muted.png`.

### Gaps

- **Q74 (quire) RESOLVED (2026-09-25).** `LevelControl { mode: LevelMode::ReadOnly }` (design/20
  §1.7's level bar): `role=progressbar`, not focusable, no handlers, and its fill transitions
  over `--t-quick --e-out`; it replaced the read-only `Slider` this gap was against, which sill
  no longer uses for the OSD.
- **Q75 (quire) RESOLVED (2026-09-25).** `Anim::OsdIn`/`OsdOut` (`--t-quick --e-out` in,
  `--t-move --e-exit` out) and the `Osd` component own the card and its fade, settled with
  `ds::settle` rather than a literal `DurationToken::Move + FRAME_SLACK` in sill.
  `Osd::on_hidden` fires once the exit has settled.
- **Q76 (quire) RESOLVED (2026-09-25).** `Osd` is one component: a single transparent Osd root
  holds it directly (`OsdMetrics::style_attr` around it for `--osd-margin`), and it paints its
  own tinted frame (`.ds-osd > .ds-frame`, the Space gradient at the root's tint). No more nested
  painted root inside a transparent one.
- **Q77 (design/22)** No `osd` rows; `AWAITING_ROWS` exempts them (a new §3.16 `osd`):

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `osd.enabled` | `OsdEnabled::{On,Off}` | `On` | shows sill's own volume and brightness changes | `20-SURFACES.md#1-7-osd`; sill `FINDINGS.md` "OSD" | proposed (2026-09-25) |
  | `osd.hold_ms` | `Ms` | `1500` | `300..=10000`; after the last change, before the fade | `20-SURFACES.md#1-7-osd` | proposed (2026-09-25) |
  | `osd.position` | `OsdPosition::{Bottom,Top}` | `Bottom` | centred along that edge | `20-SURFACES.md#1-7-osd` | proposed (2026-09-25) |
  | `osd.margin_px` (was `osd.bottom_margin_px`) | `Px` | `24` | `0..=400`; gap from the bar's reserve (top right, the default since 2026-09-25) or the dock's to the card | sill `FINDINGS.md` "OSD" | proposed (2026-09-25) |

  All Advanced until §5 places them on a page.

## M5 freeze (2026-09-25)

The control center (design/20 §1.5, design/13 §13.3.7) and the power menu (design/20 §1.8),
frozen on `m5-freeze` so four agents can fill them in parallel. Everything compiles; every pure
function a later agent fills is `todo!()` with its table test `#[ignore = "M5 fill: <agent>"]`,
so the suite stays green and each agent's first step is to delete its `ignore`s.

- **F173 Dependencies: none added.** Every new service is spoken through `zbus` (already in the
  pinned block) with hand-written proxies, or through a crate already here:
  - **Bluetooth**: hand-written `zbus` proxies for `org.bluez.Adapter1`, `Device1`, `Battery1`
    and `org.freedesktop.DBus.ObjectManager` (zbus's own `fdo::ObjectManagerProxy`). Not
    `bluer` 0.17 (BSD-2-Clause, but it runs on the `dbus` crate and links libdbus through
    `libdbus-sys`: a second D-Bus stack and a C library beside the `zbus` every other service
    uses) and not `bluez-async` (also `dbus`). There is no maintained `bluez-zbus` crate; the
    four interfaces and a dozen properties are less code than adapting one.
  - **power-profiles-daemon**: a hand-written proxy (three properties), tried on
    `org.freedesktop.UPower.PowerProfiles` (0.20 on, Fedora's) and then
    `net.hadess.PowerProfiles`.
  - **MPRIS**: hand-written proxies for `org.mpris.MediaPlayer2` and `.Player`, found by
    `ListNames` and followed by `NameOwnerChanged`. The `mpris` crate runs on `dbus`/libdbus.
  - **Session**: `logind-zbus` 5, already a dependency (the launcher's system provider).
  - **Wi-Fi**: `cosmic-dbus-networkmanager`, already the network service's (MPL-2.0, linked
    only).

  `cargo deny check licenses` is unchanged.
- **F174 The new services and their receivers.** `Services` gains `wifi`/`wifi_control`,
  `bluetooth`/`bluetooth_control`, `power_profiles`/`power_profile_control`,
  `media`/`media_control` and `session`/`session_control`. Each `spawn` is real and each
  `live::run` is a stub that returns at once, so the service is inert (its default, commands
  answering `Gone`) with no `inert()` line in `services.rs` for the agents to fight over: an
  agent fills `live::run` and nothing else changes there.
- **F175 Wi-Fi is a service of its own** (`network::wifi`), not more fields on `Network`: the
  bar draws `Network` and would repaint on every scan. `WifiCommand::{Radio, Scan,
  Connect(Ssid), Disconnect}`; `Connect` joins a *saved* network only (joining a new one needs
  a password prompt, which no milestone has yet; the page opens COSMIC Settings' Wi-Fi page
  meanwhile). `Radio` has a third state, `Blocked` (`WirelessHardwareEnabled` false), which
  disables the toggle.
- **F176 Focus is `notifications.dnd`, read.** design/22 already has `notifications.dnd`
  (`Dnd::{On,Off}`), so `sill_settings::FocusMode::{Off, DoNotDisturb}` is that key's reading
  (`focus_mode`, `with_focus_mode`), not a second copy of it. It is published by
  `sill_settings::FocusLink` (a watch that follows the settled settings, and a setter that
  rewrites `settings.toml`, so the file stays the one truth), started by the daemon with
  `spawn_focus` and given to every surface as a root context; `use_focus_mode` reads it. A mode
  beyond Do Not Disturb (Sleep, Work: "room for more") needs a key that can name it,
  `notifications.focus`, replacing `dnd`; that is a design/22 change for when a second mode is
  wanted, not a key added now.
- **F177 The session service owns logind for the power menu; the launcher keeps its snapshot.**
  `SessionAction::{Lock, LogOut, Restart, PowerOff, Suspend}`, `Offer::{Available, NeedsAuth,
  Unavailable}` (logind's `yes`/`challenge`/`no`/`na`), `SessionCommand::{Run, Refresh}`. The
  launcher's `SystemProvider` keeps its own `OnceLock` availability probe: moving it onto the
  session receiver changes the provider's constructor, which the results runner and the panel
  both call, for no behaviour anyone sees yet. `SessionAction::of_system` maps the launcher's
  `SystemCommand` (implemented, tested).
- **F178 The launcher's session rows.** `LauncherEffect` gains `Session(SessionAction)` (Lock,
  Suspend: at once) and `PowerMenu(SessionAction)` (Log Out, Restart, Shut Down: the power menu
  opens focused on that button, which is the confirmation). `run_effect` already sends
  `Session`; `PowerMenu` needs the panel's `PowerMenuLink` and output, which agent (d) passes.
  `effect_of` still maps every `System` to `Unreachable` until (d) fills it
  (`session_rows_run_or_open_the_power_menu`, ignored).
- **F179 The control center is a bar menu.** design/13 §13.3.7 has it take part in the bar's
  hover switch, so it is `BarMenu::ControlCenter` drawn in the bar's one reusable popup
  (`PopupBody::ControlCenter(ControlCenterSettings)`), not a surface of its own; the daemon
  spawns nothing for it. `sill control-center toggle` cannot open a popup from the daemon (a
  popup belongs to its bar, and an xdg popup grab wants an input serial an IPC request does not
  have), so it writes a `ControlCenterLink` ask that each bar follows for its own output.
  Whether cosmic-comp maps a popup opened that way is agent (c)'s to find out and record. The
  power profile has no module in design/22's `control_center.modules` list; its
  `SegmentedControl` sits in the Battery module (macOS puts Low Power there), alone on a
  machine without a battery.
- **F180 The power menu is one warm surface per output.** An Overlay over the whole output
  (`power_menu_layer`: anchor all, no exclusive zone, keyboard `Exclusive`, input `Whole`, blur
  `Element("power-menu-panel")`), spawned hidden with the other surfaces; the scrim is its own
  content, so no catcher is needed. Every opener (`sill power-menu`, the bar app menu's new
  "Shut Down…" item, the launcher's rows) writes one `PowerMenuLink` (implemented and tested,
  the launcher's `presence.rs` pattern with a `Preselect`); the daemon feeds it the surface's
  keyboard focus. Its panel renders nothing yet rather than `todo!()`, because the daemon
  spawns it today. The buttons are `Button{Primary}` for the focused one, `Danger` for Restart
  and Shut Down, `Secondary` otherwise; Lock is not a power-menu button (design/20 §1.8's list).
- **F181 IPC and CLI.** `Request::PowerMenu(PowerMenuRequest::{Open, Close, Toggle})` and
  `Request::ControlCenter(ControlCenterRequest::Toggle)`, wire spellings pinned in
  `sill-ipc/tests/serde.rs`; `sill power-menu {open,close,toggle}` and `sill control-center
  toggle`. The daemon's half is `crates/sill/src/power_menu.rs` (spawn and dispatch) and
  `crates/sill/src/links.rs`: the three links are installed once in a `OnceLock` and added to
  every root by `surface_root`, rather than threaded through every `spawn_missing_*` (those
  signatures were shared with the OSD work then in flight); `daemon.rs` gained a few additive
  lines.
- **F182 New keys** (no design/22 rows; `tests/keys.rs` `AWAITING_ROWS`, rows in Q82):
  `control_center.bottom_margin_px` (16: design/13 §13.3.7's "max `output_h - bar_h - 16`"),
  six `control_center.menu_bar_*` keys (F185), and a new `power_menu` domain (`[power_menu]` in
  `settings.toml`, `SettingsChange::PowerMenu`) with `power_menu.default_action` (`ShutDown`)
  and `power_menu.material` (`Sheet`).
- **F183 Things left out on purpose.** Choosing the sound output device (the volume service has
  no sink list or `SetSink`; the Sound module names the current sink only). Joining a new Wi-Fi
  network or pairing a Bluetooth device (COSMIC Settings, from each page's foot). A power-menu
  countdown (macOS's 60 s auto shut-down: not in the spec).
- **F184 Merged with the OSD.** master's OSD (FINDINGS "OSD") landed while this froze; both
  spawn one warm hidden layer per output through `daemon::surface_root`, so the power menu's
  `spawn_missing` sits beside the OSD's, and the quire gaps below are numbered after the OSD's
  Q74-Q77.
- **F185 The controls live at the top right, as on macOS (user, 2026-09-25).** The control
  center's item is the last status item, just left of the clock, and its panel hangs from the
  output's right edge, less the popup margin, not from the item (`MenuSide::OutputEdge`).
  Wi-Fi, Bluetooth, Sound, Display, Battery and Now Playing can each also be a bar item of its
  own with its own dropdown (macOS's "Show in Menu Bar"): one key per module,
  `control_center.menu_bar_{wifi,bluetooth,sound,display,battery,now_playing}:
  InMenuBar::{Show,Hide}`, Wi-Fi, Sound and Battery shown by default (which keeps today's bar),
  rather than one list key, so a module that cannot be a bar item (Focus, Appearance) cannot be
  named. The dropdowns reuse the modules' own components (`PopupBody::Module`), so the pre-M5
  volume, network and battery menus become the Sound, Wi-Fi and Battery modules; `BarMenu`
  gains `NowPlaying`, `Display` and `Bluetooth`, `BarMenu::module` maps each item to its module,
  and `control_center::MENU_BAR_ORDER` fixes their order (Now Playing, Display, Sound,
  Bluetooth, Wi-Fi, Battery, then Control Center and the clock). The new items are
  `bar::module_item::ModuleItem` and `bar::control_center_item::ControlCenterItem`, not mounted
  until agent (c) does.
- **F186 The Sound and Display modules take a level and an `on_change` (user, 2026-09-25).**
  They read no service (`Module` and the bar dropdowns wire them), and both draw through one
  `control_center::LevelModule { glyph, title, level, on_change, availability }` on quire's
  `Slider` in its default mode. quire is designing a macOS-like level style (a thick capsule
  with the glyph inside that follows the level, rubber-banding at the ends, a swell on press);
  `LevelModule` switches to it when it lands, and the slider is never styled here meanwhile.
  The OSD (moving under the bar at the top right, its own follow-up) is to reuse `LevelModule`.

### The M5 fill-in: four agents, disjoint files

Every agent reads CONVENTIONS §0 and §1 (the freeze rule: fill bodies, add private helpers and
tests; never change a frozen signature, public field or variant; stop and report instead),
ORCHESTRATION "Coherence rules", design/20 §1.5 or §1.8 and design/13 §13.3.7, and this
section. Everyone runs, before reporting:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features sill/debug
./scripts/check-boundary.sh
cargo deny check licenses
```

(`cargo fmt -p <crate>` while the wave runs.) Files not listed under an agent are read-only to
all four: in particular `sill-services/src/{lib.rs,services.rs}`, `sill-surfaces/src/hooks/**`,
`sill-settings/src/**` but `focus_link.rs`, `sill-ipc/**`, and `crates/sill/src/{daemon.rs,
cli.rs,links.rs,outputs.rs}`.

**(a) svc-session-bt: Bluetooth, power profile, session.**
- Owns: `crates/sill-services/src/bluetooth/**`, `power_profile/**`, `session/**` (new private
  files in them, e.g. `bluetooth/bluez.rs` for the proxies), new
  `crates/sill-services/tests/{bluetooth_live,power_profile_live,session_live}.rs`.
- Frozen: each `spawn`; `BluetoothCommand`, `Bluetooth`, `BtDevice`, `BtAddress`, `BtPower`,
  `BtKind`, `BtConnection`, `BtPairing`, `BtBattery`, `BluetoothEvent`, `reduce_bluetooth`,
  `listed_devices`, `device_kind`; `ProfileCommand`, `PowerProfiles`, `Profile::{ALL,
  from_daemon, daemon_word}`, `ProfileBus`, `ProfileDegraded`, `ProfileEvent`,
  `reduce_power_profiles`; `SessionCommand`, `Session::offer`, `SessionAction::{POWER_MENU,
  of_system}`, `Offer::from_logind`, `SessionEvent`, `reduce_session`.
- Done: the ignored tests in those modules pass un-ignored; each `live::run` seeds a real
  value, follows changes (BlueZ `InterfacesAdded`/`Removed` and `PropertiesChanged`; ppd's
  properties; logind re-probed on `Refresh` and on `PrepareForSleep(false)`), and carries out
  its commands (`Connect` publishes `Connecting` first); without the bus or daemon it returns
  and the receiver stays default. Live tests, `#[ignore]`, read-only on this machine (never
  suspend, reboot or power off from a test): the adapter and its devices, the active profile,
  the offers.
- Verify: the gates, plus `cargo test -p sill-services -- --ignored` for its live tests on the
  machine, reporting what each saw.

**(b) svc-media-wifi-focus: Now Playing, Wi-Fi, focus.**
- Owns: `crates/sill-services/src/now_playing/**`, `crates/sill-services/src/network/wifi/**`,
  `crates/sill-settings/src/focus_link.rs` (the `spawn_focus` body),
  `crates/sill-settings/tests/focus.rs`, `crates/sill-services/Cargo.toml` (an `[[example]]`
  entry only), a new `crates/sill-services/examples/mpris_player.rs` (a test MPRIS player,
  `badge_emitter`'s pattern), new `crates/sill-services/tests/{media_live,wifi_live}.rs`.
- Frozen: `MediaCommand`, `MediaVerb`, `Media::current`, `MediaPlayer`, `PlayerId`, `Playback`,
  `Track`, `Art`, `Micros`, `Timeline`, `Abilities`, `MediaEvent`, `reduce_media`, `ArtSource`,
  `art_source`, `art_cache_name`; `WifiCommand`, `Wifi`, `Radio`, `Ssid`, `AccessPoint`,
  `Security`, `Association`, `Saved`, `WifiEvent`, `reduce_wifi`, `listed_networks`;
  `FocusLink::{new, fixed, mode, set}`, `Delivery`, `spawn_focus`.
- Done: the ignored tests pass (`art_cache_name` must be stable across builds, so not std's
  `DefaultHasher`); MPRIS players found and followed, `Position` read on a status change and on
  `Seeked`, `Press` carried out; remote art downloaded once into `$XDG_CACHE_HOME/sill/art/`
  with no new crate (`curl` as a child process, which Fedora ships, or stop and propose a
  dependency); Wi-Fi radio, access points (`AccessPointAdded`/`Removed`, `LastScan`) and saved
  connections' SSIDs followed, commands carried out; `spawn_focus` follows the feed and writes
  the file (`tests/focus.rs`). Live tests `#[ignore]`: the example player becomes current and
  obeys PlayPause; a scan returns access points.
- Verify: the gates, plus the ignored live tests on the machine.

**(c) control-center: the popup, its modules and the bar item.**
- Owns: `crates/sill-surfaces/src/surfaces/control_center/**`; in the bar,
  `surfaces/bar/{menus,content,geometry,popup,menu_popup,session,plan,status,volume,network,
  battery,mod}.rs`, `surfaces/bar/module_item.rs` and `surfaces/bar/control_center_item.rs`; `crates/sill-surfaces/src/style/control_center.css`
  (new) and `style/bar_popup.css`; `crates/sill-surfaces/tests/support/fake.rs` (feed senders
  for the new services); new `tests/{control_center_render,control_center_lint}.rs`;
  `examples/bar_demo.rs`; `dev/accept-control-center.sh`.
- Frozen: `ControlCenter { output, on_close, on_page }`, `Module`, every `*Module` and
  `PowerProfileControl`, `WifiNetworks`/`BluetoothDevices { on_back }`, `page::{CcPage, Detail,
  PageInput, Slide, PageOut, step_page}`, `geometry::{Span, span, Placed, place, height_cap,
  estimate_size}`, every `labels` function, `ControlCenterLink`/`ControlCenterAsk`,
  `LevelModule`, `SoundModule { level, device, on_change, availability }`, `DisplayModule {
  level, on_change, availability }`, `menu_bar::{MENU_BAR_ORDER, in_menu_bar,
  menu_bar_modules}`, `ModuleItem { output, module }`, `ControlCenterItem { output }`;
  crate-private but agreed: `BarMenu::{ControlCenter, NowPlaying, Display, Bluetooth}`,
  `BarMenu::module`, `MenuSide::OutputEdge`, `PopupBody::{ControlCenter, Module}`.
- Done: `ControlCenterItem` (quire `IconButton{Status}`), the last status item left of the
  clock, opens the control center, hung from the output's right edge; `ModuleItem`s per
  `menu_bar_modules`, each dropping down its module (replacing `VolumeItem`, `NetworkItem` and
  `BatteryItem` and their panels); hover switch across all of them; the popup root gets `Services`,
  `SettingsFeed` and `FocusLink` (from the bar's `try_use_context`) and calls
  `ds_native::measure::provide()`; the bar follows `ControlCenterLink` for its output; the
  popup is sized by `estimate_size` and repositioned on a page change; modules in
  `control_center.modules` order on `place`'s grid; the Wi-Fi and Bluetooth pages slide (quire
  motion only: Q80); Escape unwinds; only quire components (the SSR class test and
  `ds::lint::stylesheet`, as the bar's tests do); every ignored test in the module passes.
- Verify: the gates; `dev/accept-control-center.sh` on the nested compositor (`dev/nested.sh`,
  `sill run --features debug` with a scratch `XDG_CONFIG_HOME`, as `accept-launcher.sh` does):
  (1) `sill debug click` on the item opens `sill-bar-<out>/popup` (a capture succeeds), a
  second click closes it; (2) `sill control-center toggle` opens and closes it (or records the
  grab refusal and exits 2 naming it); (3) captures of the grid and of the Wi-Fi page into
  `dev/out/control-center*.png`; (4) the bar idles at 0 frames for 5 s after closing.

**(d) power-menu: the surface, its keys, the launcher's rows.**
- Owns: `crates/sill-surfaces/src/surfaces/power_menu/**` (the implemented `presence.rs` and
  `layer.rs` may change inside, never in signature); `surfaces/launcher/{activate,run,
  events}.rs`; `crates/sill-services/src/providers/system.rs` (a confirm row's primary becomes
  its `System(command)`, since the power menu is now the confirmation);
  `crates/sill/src/power_menu.rs`; `crates/sill-surfaces/src/style/power_menu.css` (new); new
  `tests/{power_menu_render,power_menu_lint,power_menu_keys}.rs`; `dev/accept-power-menu.sh`.
- Frozen: `PowerMenu { output }`, `PowerMenuLink`, `PowerMenuShown`, `PowerMenuOpening`,
  `Preselect`, `power_menu_layer`, `panel_region`, `PANEL_ID`, `keys::{PowerMenuKey,
  PowerMenuState, PowerMenuOut, initial, step}`, `labels::{action_label, button_variant}`,
  `LauncherEffect::{Session, PowerMenu}`, `PowerMenuRequest`, the CLI.
- Done: the panel follows the link onto its visibility; a `Ds` root with a quire `Scrim` and
  `Sheet` (`peek-in`), four `Button`s per `button_variant`, unavailable per `Session::offer`,
  `Kbd` hints (Return, Esc); `step` on the keys; a chosen action sends `SessionCommand::Run` and
  closes; a focus loss closes; the launcher's rows per F178; the bar's "Shut Down…" (already
  wired) opens it; the ignored tests pass.
- Verify: the gates; `dev/accept-power-menu.sh` on the nested compositor: (1) `sill power-menu
  open` shows `sill-power-menu-<out>` (capture into `dev/out/power-menu.png`), `close` hides
  it, `toggle` twice round-trips; (2) the bar app menu's "Shut Down…" opens it (`sill debug
  click`); (3) Escape closes it (by injected key if shell-host offers one, else a manual check
  in `docs/manual-checks.md`); (4) 0 frames idle while hidden. Never confirm an action in the
  script: the nested session shares logind with the machine.

### Gaps found in the M5 freeze

- **Q78 (quire)** A control center module tile: design/13 §13.3.7's `--r-tile` 12 tile with a
  glyph on an accent disc when on, a title, a status line, an on/off state, an optional chevron
  to a detail pane, and a row-spanning variant for sliders. Nothing in the catalogue has that
  shape (`AccountTile` is the mail account plate).
- **Q79 (quire)** A settings-style list row for networks and devices: leading glyph, title,
  detail, trailing marks (lock, check, spinner), selectable and pressable, outside a `Menu`.
  `ListRow` is the mail row (name, subject, snippet, time, star), and `MenuEntry::Row` lives
  only inside a `Menu`.
- **Q80 (quire)** A pane switcher that plays `slide-l` / `slide-r` between a root pane and a
  detail pane in place (design/13 §13.3.7 "Detail"): `Anim::SlideL`/`SlideR` exist, but
  `use_entrance` is crate-private and consumers may not write keyframes (rule 4).
- **Q81 (quire)** Glyphs: media transport (`Play`, `Pause`, `SkipBack`, `SkipForward`),
  `LogOut` and `Restart` for the power menu, and Bluetooth device kinds (`Headphones`,
  `Speaker`, `Mouse`, `Gamepad`, `Phone`; `Keyboard` and `Monitor` exist).
- **Q83 (quire, in design)** The level control style F186 waits for: a thick capsule with the
  glyph inside following the level, rubber-banding at the ends, a swell on press. `LevelModule`
  is the one place that adopts it.
- **Q82 (design/22)** The rows `AWAITING_ROWS` exempts (F182), for §3.13 and a new section
  after §3.16 `osd` (which `tests/keys.rs` then reads without change, since everything up to
  "## 4. Rust shape" is parsed); §2's file table and §4.4's `ShellFile` gain `power_menu`:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `control_center.bottom_margin_px` | `Px` | `16` | 0..=64; the panel scrolls past `output_h - bar_h - this` | `13-BEHAVIOUR-menus-windows.md#13-3-7-control-center` | proposed |
  | `control_center.menu_bar_wifi` | `InMenuBar::{Show,Hide}` | `Show` | Wi-Fi as its own bar item and dropdown (macOS "Show in Menu Bar") | `20-SURFACES.md#1-5-control-center-spec-tier-1`; sill FINDINGS F185 | proposed (user, 2026-09-25) |
  | `control_center.menu_bar_bluetooth` | `InMenuBar::{Show,Hide}` | `Hide` | Bluetooth as its own bar item | same | proposed (user, 2026-09-25) |
  | `control_center.menu_bar_sound` | `InMenuBar::{Show,Hide}` | `Show` | Sound as its own bar item (today's volume item) | same | proposed (user, 2026-09-25) |
  | `control_center.menu_bar_display` | `InMenuBar::{Show,Hide}` | `Hide` | Display (brightness) as its own bar item | same | proposed (user, 2026-09-25) |
  | `control_center.menu_bar_battery` | `InMenuBar::{Show,Hide}` | `Show` | Battery as its own bar item (today's battery item) | same | proposed (user, 2026-09-25) |
  | `control_center.menu_bar_now_playing` | `InMenuBar::{Show,Hide}` | `Hide` | Now Playing as its own bar item | same | proposed (user, 2026-09-25) |
  | `power_menu.default_action` | `PowerMenuDefault::{LogOut,Restart,ShutDown,Suspend}` | `ShutDown` | the button Return presses when the menu opens | `20-SURFACES.md#1-8-power-menu-spec-tier-1` | proposed |
  | `power_menu.material` | `PanelMaterial::{Sheet,Popover}` | `Sheet` | Sheet over Scrim (design/20 §1.8) | `20-SURFACES.md#1-8-power-menu-spec-tier-1` | proposed |

  All Advanced (on the Appearance page, F10's rule); the six `menu_bar_*` keys are the macOS
  Control Center page's own and are candidates for a Basic page when design/22 §5 gets one.
  design/20 §1.1's bar content ("tray, volume, network, battery, clock") becomes "tray, the
  module items `control_center.menu_bar_*` shows, the control center, clock".

## M5 power menu (lane d, 2026-09-25)

The power menu (design/20 §1.8) filled on `m5-d`. Findings are numbered from F200/Q90/G30 so
the four M5 lanes cannot collide.

- **F200 The menu.** A transparent `Ds` root (the active Space's look) holds the room, a box as
  large as the output that fades out as a whole; in it a `Ds` in `power_menu.material` holds
  quire's `Sheet` (its own full-output scrim, `peek-in` `--t-big` `--e-spring`, the scrim's
  `fade` `--t-move`) with a Power glyph, a macOS-style question for the focused button ("Shut
  down this computer now?") and what it does ("Your apps will quit and the computer will turn
  off."), the four `Button`s in `SessionAction::POWER_MENU` order per `button_variant`
  (unoffered ones `Availability::Disabled`), and `Kbd` hints (← → Choose, ↵ <focused>, Esc
  Cancel). The sheet is keyed by the opening, so each opening replays the entrance and starts
  from `initial`. The focused button is `Primary` and also has the DOM focus (`ds::focus_soon`
  on its `mounted` element), so quire's keyboard ring moves with the arrows.
- **F201 Every way out leaves first.** `sheet.rs` (pure, table-tested) wraps the frozen `step`:
  Escape (through the `Sheet`'s own handler or the root's), a scrim click and a chosen action
  all move the sheet to `Leaving`; the room plays quire's `menu-out` (`--t-quick --e-exit`,
  `Anim::MenuOut`), timed by `ds::use_motion_timer(Anim::MenuOut)`; once it settles the menu
  closes and only then does a chosen action run, so a suspend or a polkit prompt never finds
  the sheet still up. Input while leaving is ignored, so Enter's key and the button's click
  cannot run an action twice. A close from outside (`sill power-menu close`, a focus loss) hides
  at once.
- **F202 The session behind a trait.** `power_menu::backend::SessionBackend` (`offers`,
  `refresh`, `run`) with `LiveSession` (the session service: `Run`, and `Refresh` on each
  opening) and `DryRunSession` (every action offered; a chosen one is kept, printed as
  `power_menu.dry_run would=<Label>` and appended to a record file). The panel takes a
  `PowerMenuBackend` root context when there is one, else the live session from `Services`.
  **Dry run:** a debug build (`--features debug`) started with `SILL_POWER_MENU_DRY_RUN=<file>`
  gives every power menu the dry run (`crates/sill/src/power_menu.rs`; a release build ignores
  the variable); each surface prints `power_menu.backend=<live|dry-run>` as it mounts. Every
  test uses the dry run. The launcher's Lock and Suspend rows still go straight to the session
  service (F178: they need no confirmation); nothing in this lane's tests or acceptance
  chooses them.
- **F203 The launcher's rows.** `effect_of` maps `System(Lock|Suspend)` to `Session`,
  `System(LogOut|Restart|ShutDown)` to `PowerMenu` (the power menu opens on that button, which
  is the confirmation) and `Reload` to `Unreachable`; `run_effect` takes an `EffectFrom`
  (surface, output, the `PowerMenuLink`) and opens the menu. The system provider's rows all
  activate on Enter and carry no "confirm" action any more.
- **F204 The blur corner is 14.** `.ds-sheet` rounds with a plain `border-radius:
  var(--r-panel)` (a circle of 14, not a squircle), and the `power-menu-panel` element is the
  sheet's whole box (`tests/power_menu_render.rs` checks the two rects are equal), so
  `PANEL_BLUR_RADIUS` is 14.
- **F205 A `.ds` root is as tall as its content.** The sheet floats in its `Ds`'s overlay
  (`position:absolute; inset:0` of the root), and a root holding only floating content is 0 px
  tall: the scrim was 0 px and the sheet's `max-height: calc(100% - 72px)` collapsed it. An
  in-flow `.sill-power-menu-floor` (`height:100vh`) gives the root the output's height.
- **F206 The acceptance (`dev/accept-power-menu.sh`, all six steps pass).** Warm and hidden;
  `open` shows the sheet over the scrim (`dev/out/power-menu.png`, `dev/out/power-menu-cosmic.png`),
  `close` hides it, `toggle` twice round-trips; Right, Left, Left move the focus Shut Down →
  Suspend → Shut Down → Restart (`dev/out/power-menu-cosmic-restart.png`); Enter records exactly
  `power_menu.dry_run would=Restart` and closes; Escape closes recording nothing; 5 s hidden,
  0 frames. The script proves the dry run from the daemon's log before any key and never
  presses Enter without that proof.
- **G30 No keyboard on the nested compositor.** With the nested window unfocused on the host,
  cosmic-comp sent the power menu's `Exclusive` layer no `wl_keyboard.enter` (no
  `KeyboardFocus` event, so no `power_menu.keyboard=held`), and `wtype` keys (the
  virtual-keyboard protocol) never reached it; shell-host injects no keys (G20). So the debug
  key path: a debug build started with `SILL_POWER_MENU_KEYS=<fifo>` too feeds each word
  written to it (`right`, `enter`, ...) to every power menu as that key
  (`power_menu::script`, the launcher demo's `LauncherScript` pattern). The daemon refuses it
  without the dry run, so a scripted Enter can never reach logind. A real keyboard on the real
  session remains a manual check.
- **G31 Not verified live: the bar app menu's "Shut Down…".** `sill debug click` on the app
  menu opens it (the popup captures with "Shut Down…"), but a click on the item did not open
  the power menu and a pointer over it did not highlight it; the bar popup is lane (c)'s. The
  pick is unit-tested (`bar/pick.rs`, `BarPick::PowerMenu` → `link.open`).

### Gaps found in the power menu (quire)

- **Q90 (quire), closed** ("Adopt quire v0.1.9") `Sheet` has no leaving state: it unmounts at once (`use_entrance` is
  entrance-only). The power menu fades its whole room with quire's `menu-out` instead; the
  sheet's own exit (a reverse `peek-in`, the scrim fading at `--t-move`) belongs in quire.
- **Q91 (quire), closed** ("Adopt quire v0.1.9") A centred `Sheet`: it sits at Peek Center's 36 px top inset. macOS's
  shut-down dialog (and every confirm alert) is centred on the screen; a `SheetPlacement::Center`
  would let the power menu sit there.
- **Q92 (quire), closed** ("Adopt quire v0.1.9") A dialog-sized `Danger` button: `Danger` is `Mini` at rest, so Restart and
  Shut Down (F180's frozen `button_variant`) draw smaller than Log Out and Suspend beside them.
  A destructive action in a dialog wants `Secondary`'s geometry, red on hover.
- **Q93 (quire), closed** ("Adopt quire v0.1.9") A disabled look for `Button` (O-1: `aria-disabled` only drops the handler), so
  an action logind does not offer does not read as pressable.
- **Q94 (quire), closed** ("Adopt quire v0.1.9") A full-surface `Ds` root (a `fill` prop, or overlay bounds from the viewport)
  so a root holding only a `Sheet` need not add F205's floor.
- **Q95 (quire), closed** ("Adopt quire v0.1.9") The arrow key caps (`Key::Left`, `Key::Right`, O-2) draw as a short dash at
  `KbdSize::Small` in the shell font; the glyphs want sign-off and a font that has them.
- Q81's `LogOut` and `Restart` glyphs would let the buttons carry icons; they carry none
  meanwhile (macOS's do not either).

## M5 control center (lane c, 2026-09-25)

The control center (design/20 §1.5, design/13 §13.3.7), its module bar items and
`dev/accept-control-center.sh`, filled on `m5-c`. Findings are numbered from F220/Q100/G40 so
the M5 lanes cannot collide.

- **F220 The panel is quire's control-center parts.** `ControlCenter` draws the modules in
  `control_center.modules` order on quire's `ModuleGrid`: Wi-Fi, Bluetooth and Focus as
  `ModuleTile`s (a press toggles; Wi-Fi's and Bluetooth's chevrons open their panes; Busy maps
  to on), and the full-width modules (Display, Sound, Now Playing, Appearance, Battery) on a
  plate of the tile's own tokens (`.sill-cc-module`, Q100). The Wi-Fi and Bluetooth panes are
  quire `SettingsRow` lists (a check on the one in use, else a lock on a secured network)
  behind quire's `PaneSwitcher` (`slide-r` in, `slide-l` back, the leaving pane out of the
  flow); the page and which pane was last opened are sill's (`page::step_page`, pure). Now
  Playing takes quire's `Play`/`Pause`/`SkipBack`/`SkipForward`, the device rows `Headphones`,
  `Speaker`, `Mouse`, `Gamepad`, `Phone`. The stand-ins built while Q78-Q81 were open are
  deleted with their CSS.
- **F221 The levels are quire's `LevelControl`.** `LevelModule` (Sound, Display) is a title and
  the percentage over quire's `LevelControl` (Q83, landed): `Interactive`, `Tick::Quiet`, the
  default `LevelLook::Capsule` chosen in `level.rs`'s one `LOOK` (the user's pick may change
  it), the glyph mapped from the module's (`Sun` to `Brightness`, the rest to the speaker,
  whose waves follow the level; the frozen `SoundModule` is told no mute). Sound's title is the
  sink's name (F186); a sink list is still left out (F183).
- **F222 The items at the top right.** The bar's status area is the tray, then
  `menu_bar_modules` as `ModuleItem`s (Sound, Wi-Fi, Battery by default; Now Playing shows only
  while a player exists, Battery only with a battery), then `ControlCenterItem`, then the clock.
  The old volume, network and battery items and the volume panel are gone: those items drop
  down their modules (`PopupBody::Module`). A module's own dropdown opens its detail directly
  (design/13 "Module bar items"): Wi-Fi and Bluetooth their panes without a back button, the
  others their module on the popover card with no second plate. The control center's item is
  quire's `Settings` glyph (Q103).
- **F223 Hung from the output's edge.** `MenuSide::OutputEdge` anchors to the bar's last column
  with the offset `(SIDE - EDGE_GAP, 6)` = (20, 6), so the card's right edge is 8 px inside the
  output wherever the item is; its positioner allows no slide or flip (`Adjust::NONE`), since
  the shadow room reaches past the output's edge on purpose and sliding it back would pull the
  card in by the room.
- **F224 The popup is sized by estimate and resized on a page change.** `control_center::fit`
  (pure) turns the settings, the page and the lists' row counts into the popup's size
  (`estimate_size`, capped by `height_cap`); the panel reports each page change on its own
  channel (`PopupLink::pages`), and the bar's session swaps the view, which repositions the one
  popup. Showing a menu afresh starts the control center on its grid. The Wi-Fi and Bluetooth
  row counts are read from the services' receivers when the view is made, not followed: a scan
  must not re-render the bar (F175; following them cost 2 frames of the 5 idle seconds). The
  popup root gets `Services`, `SettingsFeed`, the `FocusLink` (a fixed Off one when the bar has
  none) and the cap (`PanelCap`, past which the panel scrolls).
- **F225 Presses reach a bar popup now.** The popup's frame was absolutely placed with insets;
  neither `#main` nor `.ds` has a height (F205), so every box from `html` down was 0 px tall
  and Blitz's hit test never descended to the card: no press in any bar popup did anything
  (G31 is this). The frame is now in flow at `100vw` x `100vh` with the room as its padding and
  the `Ds` root filling it (a one-cell grid); `tests/control_center_render.rs` clicks a tile, a
  chevron and a network row through the same frame in a Blitz harness, and live debug clicks
  in the popup now land: G31 passes by hand on the nested session (a debug build with the power
  menu's dry run: `sill debug click` on the app name, then on "Shut Down…", mapped
  `sill-power-menu-<out>`; `power-menu close`, nothing chosen). `dock_popup.css` and the tray's
  popup were not looked at; they may have the same frame.
- **F226 Escape.** The panel element takes the keyboard as it mounts and again after a page
  change takes the focused chevron or back button away (`ds::focus_soon`), so Escape on a pane
  goes back and on the grid reaches the popover, which closes (harness-tested). Live, `wtype -k
  Escape` did not reach the popup: a popup opened by a debug click grabs with no real input
  serial, so cosmic-comp gives it no keyboard (G30's kind). A real keyboard is a manual check.
- **F227 `sill control-center toggle`.** The bar follows `ControlCenterLink` for its output and
  toggles as a click would, placing the panel from the status box when its item has not been
  measured. cosmic-comp maps the popup opened this way (shell-host grabs with the last serial
  it has), it stays mapped, a second toggle closes it, and an outside click on the bar closes
  it.
- **F228 The acceptance (`dev/accept-control-center.sh`, all six steps pass).** The item click
  opens the 376-wide popup, the card's shadow 5 px from the right edge, and a second click
  closes it (`dev/out/control-center-cosmic.png`); an outside click closes it; the Wi-Fi
  chevron swaps the popup to the pane's size (798 to 1018 on this machine's scan,
  `dev/out/control-center-wifi-cosmic.png`); the Wi-Fi bar item opens the same pane size
  directly; the toggle as F227; 0 frames in 5 idle seconds (the window keeps clear of the
  clock's minute). Nothing in it presses a switch or a slider: the only request is the pane's
  scan.

### Gaps found in the control center

- **Q100 (quire), closed** ("Adopt quire v0.1.9") A full-width module with content of its own: `ModuleTile` holds a glyph, a
  title and a status, not children, so the level, Now Playing, Appearance and Battery modules
  sit on sill's `.sill-cc-module` plate (the tile's `--r-tile`, `--surface-2` and hairline).
  A content-holding `ModuleTile` (or a plate of `LevelControl`'s own, as the OSD card has)
  would remove it.
- **Q101 (quire), closed** ("Adopt quire v0.1.9") `AppearancePicker` at the control center's width: the Motion row's five
  segments overflow the 296 px module (Reduced is clipped). A compact picker (Theme and Accent
  only, or segments that shrink) would fit a 320 panel.
- **Q102 (quire), closed** ("Adopt quire v0.1.9") `ModuleGrid` is two columns at gap 8, fixed: `control_center.grid_columns`
  and `grid_gap_px` only size the estimate now. `columns` and `gap` props would honour them.
- **Q103 (quire), closed** ("Adopt quire v0.1.9") A control center glyph (macOS's two switches); the item uses `Settings`.
- **Q104 (quire, CONSUMING), closed** ("Adopt quire v0.1.9") A popup document's frame must be in flow with a height (F225):
  worth a line beside `ds_native::measure::provide` in the bar-gaps notes, since every host
  that floats a card in a transparent `Ds` meets it.
- **Q105 (quire, for the look pick), closed** ("Adopt quire v0.1.9": the light well is darker) On the light scheme the `Capsule` level's fill (the
  material's bright ink) sits pale on the module's `--surface-2` plate: in
  `dev/out/control-center-cosmic.png` the Sound capsule's filled half reads only faintly
  against its track. Worth weighing when the user picks the level look (`level.rs`'s `LOOK`).
- **G40 (lane b)** On this machine the Wi-Fi pane lists the joined network as "Connecting…"
  while the tile reads it as joined: `AccessPoint::association` seems to lag the device's
  state. Not changed here (the service is lane b's).
- **G41** `bar.css` still styles `.sill-volume-panel`, which nothing draws any more (the Sound
  module replaced the volume panel); `bar.css` is not this lane's file.

## Adopt quire v0.1.9 (2026-09-25)

The power menu, the control center and the popups on quire's sheet parts and control center
parts 2 (quire CONSUMING "Sheet and modal parts", "Control center parts 2", §2's root-frame
rule), on `adopt-v019`. Findings are numbered from F240/Q110/G50.

- **F240 The power menu is one Viewport root with quire's own exit (Q90, Q91, Q94).** `Ds {
  material: power_menu.material (Sheet by default), extent: RootExtent::Viewport }` holds `Sheet
  { placement: Centre, scrim: ScrimStrength::Modal, shown, on_hidden }`: the outer transparent
  root, the room's `menu-out` fade and its motion timer, and `.sill-power-menu-floor` are gone.
  Escape, a scrim click or a chosen action set `shown` to `Hidden`; quire plays `sheet-out` and
  the scrim's fade, and at `on_hidden` the menu closes and only then does a chosen action run
  (`sheet.rs`'s `Settled`, as before). While it leaves the link says `Showing::Leaving` (the
  surface stays mapped); `open` the same way again then shows it again (`Showing::Up`,
  `follow.rs`'s `Reshow`), the sheet enters again and `on_hidden` does not run
  (`power_menu_keys.rs` `opened_again_while_leaving_it_enters_again_and_stays`). `toggle` on a
  leaving menu shows it again. The keys are taken on the panel inside the sheet (the sheet sits
  in the root's overlay, outside any wrapper of sill's); the panel's `data-look` is the phase.
  The sheet is centred both ways and the root is the viewport (`power_menu_render.rs`).
- **F241 Even buttons and disabled actions (Q92, Q93).** `Danger` takes `size:
  Some(ButtonSize::Regular)` (`labels.rs` `button_size`), so Restart stands level with Log Out
  and Suspend; an action logind does not offer is `Availability::Disabled` and now carries
  `disabled="true"` beside `aria-disabled` (tested) and draws at .35. Primary (Shut Down) is 2 px
  shorter than Secondary and Danger: Secondary has a hairline border and Primary none (Q110).
- **F242 A dry run on logind's offers.** A debug build started with
  `SILL_POWER_MENU_DRY_RUN_OFFERS=logind` beside the dry run offers what the session service
  reads from logind (`DryRunSession::following`, read only; a choice is still only recorded and
  the dry run holds no sender to logind at all). Each opening logs `power_menu.offers=`.
  `dev/accept-power-menu.sh` runs on it, so the shot shows this machine's refusals greyed, and
  its arrows step expects the next offered button right of Shut Down. On the nested session
  logind offers Restart and Shut Down only: Suspend is refused (`CanSuspend` "no") and Log Out
  is unavailable there too (the nested session has no logind session of its own to terminate).
  All six steps pass (`dev/out/power-menu-cosmic.png`: centred, the modal scrim about 0.77 of the
  old one's brightness on the wallpaper, Log Out and Suspend greyed).
- **F243 The Kbd arrows (Q95).** `←` and `→` now draw as arrows in bordered caps, but at
  `KbdSize::Small` the glyph is about 4 px wide and still reads close to a dash at 1x (Q111).
- **F244 The wallpaper root is Viewport.** `Ds { extent: RootExtent::Viewport }`, and
  `.sill-wallpaper` fills it with `position:absolute; inset:0` (Taffy places it against the root,
  a browser against the viewport: the same box) instead of `100vw`/`100vh`. The paint test now
  checks both corners.
- **F245 The control center on ModulePanel (Q100-Q103, Q105).** `module_box.rs` and
  `.sill-cc-module` (and its dropdown rule) are gone: every full-width module is a `ModulePanel`,
  `PanelPlate::Bare` in a module's dropdown (the dropdown provides the plate as a context,
  `modules::use_plate`). `LevelModule` passes the glyph, the title and `trailing: "{percent}%"`;
  Battery the same when there is a battery; `.sill-cc-level*`, `.sill-cc-value`,
  `.sill-cc-glyph` are gone. Appearance takes `PickerLayout::Compact` (Motion's five segments fit
  the 296 px module, `dev/out/control-center-cosmic.png`). `ModuleGrid` takes `columns` and `gap`
  from `control_center.grid_columns`/`grid_gap_px` and `padding: Px(0.0)`: the panel keeps
  `grid_padding_px` since it pads the detail panes too, so the layout is unchanged. The bar item
  is `Icon::Switches`. The light level well is quire's darker one now (Q105).
- **F246 The dock and tray popups broke the root-frame rule, now fixed (Q104).** The dock menu's
  frame was absolutely placed with insets and the tray's root held only the floating menu: every
  box from `html` down was 0 px (harness-measured). The dock frame is now the bar popup's: in
  flow, `100vw` x `100vh`, a one-cell grid, the shadow room as padding (`frame_style` writes
  padding); the tray's root is `RootExtent::Viewport`. Their documents moved to
  `dock/menu_document.rs` and `bar/tray_menu.rs` so a harness can mount them without a surface;
  each has a test that measures the heights and presses an entry, and both tests fail on the old
  frames.
- **F247 Real input (`dev/accept-real-input.sh`, all six steps pass).** On shell-host's headless
  KWin (`kwin-virtual.sh`, `fake-input.sh`; shell-host F52), with the dry run proven first: a
  real click on the control center item opens it and real Escape closes it; a click opens it
  again and a click outside closes it; the power menu holds the real keyboard
  (`power_menu.keyboard=held`, which the nested cosmic-comp never gave: G30), real Right moves
  Shut Down to Suspend and Left Left to Restart, real Escape closes it recording nothing, and a
  real click on the scrim closes it recording nothing. Enter is never pressed and no menu button
  is clicked. The headless KWin is stopped at the end.

### Gaps found adopting v0.1.9

- **Q110 (quire)** `Button{Primary}` has no border and `Secondary`/`Danger` (Regular) a hairline,
  so in a row the focused Primary is 2 px shorter than its neighbours (36 vs 38 at 1x), and the
  row's height shifts by a pixel as the focus moves. A transparent border of the same width on
  Primary would level them.
- **Q111 (quire)** The `Kbd` arrow caps at `KbdSize::Small` draw the arrow at about 4 px: legible
  as an arrow only when zoomed. A larger glyph size inside the small cap would read.
- **G50 (shell-host)** Each `fake-input.sh` client is a new KWin fake input device, and KWin
  re-announces the seat's capabilities for it (pointer removed and added). shell-host makes a new
  `wl_pointer` on each announcement without releasing the old one, so a press reaches the
  surfaces once per pointer: after a few clients a click on the bar opened and shut the popup in
  one go (seen with `WAYLAND_DEBUG`: three `wl_pointer.button` for one press, two
  `xdg_popup.grab`). `accept-real-input.sh` keeps one client for the whole run. A real seat that
  loses and regains its pointer (a mouse unplugged) would do the same.
- **G51** On the headless KWin `sill quit` does not end the daemon (it answers, then stays up);
  the script ends it after 5 s. On the nested cosmic-comp it quits as before. Closed below
  ("G51: `sill quit` ends the daemon on headless KWin").
- **G52 (kwin-virtual.sh)** `kwin-virtual.sh stop` kills every process whose environment names
  the private bus, so a script that sourced `.kwin-env` and exported it is killed by its own stop
  call; `accept-real-input.sh` gives the bus to the daemon only.

## G51: `sill quit` ends the daemon on headless KWin (2026-09-25)

`gdb -p <pid> -batch -ex "thread apply all bt"` on the stuck daemon (headless KWin, no other
change) found the main thread past `host.run` in `daemon.rs`, inside `Drop` for
`tokio::runtime::runtime::Runtime` (`BlockingPool::shutdown` waiting on a oneshot receiver for
every blocking-pool task to finish), and a `tokio-rt-worker` thread inside a `spawn_blocking`
task still running `std::process::Command::output`'s `poll()`, called from
`sill_services::displays::sink::CosmicRandr::heads` (`crates/sill-services/src/displays/sink.rs`)
by way of `scale_pass` and `displays::run::Service::rescan_outputs`
(`crates/sill-services/src/displays/run.rs`). `pgrep`/`ps` confirmed it: a `cosmic-randr list
--kdl` child, still alive, spawned by the daemon at startup and never reaping.

The module doc's assumption ("under a compositor without `zwlr_output_manager_v1` (KWin) it
fails") was wrong: `cosmic-randr` hangs on KWin rather than failing promptly, so
`Command::output` (no bound of its own) waits on its stdout pipe forever. `displays`' first
`rescan_outputs` (fired from the daemon's very first policy send) is stuck on that
`spawn_blocking` task for the rest of the session — which, besides parking `displays` (no
further display command is ever answered under KWin, in this build, before this fix), also
parks `tokio::runtime::Runtime`'s plain `Drop`, called implicitly when `daemon::run` returns
after `host.run` (`ctx.exit()`): it waits forever for every blocking-pool thread, including this
one, to finish. `sill quit` answers over IPC (the reply is sent from `ctx.exit()`'s handler,
before `host.run` even returns) while the process behind it never exits.

Not shell-host's: `Host::run` (shell-host's own function) already shuts its own, separate,
internal runtime down in a bounded way (`runtime.shutdown_background()`, non-blocking) and
clears every surface — and with it every clone of `sill_services::Services` the surfaces
held — before it returns; the daemon's *own* `runtime` (built in `daemon::run`, passed to
`sill_services::Services::spawn`) is the one left to an unbounded `Drop`. Not G50 or G52 either:
G50 is shell-host's fake-input pointer double-delivery and G52 is `kwin-virtual.sh stop`'s kill
pattern catching a script that sourced its env; neither touches how or whether the daemon process
itself exits, and this repro reliably hung with a single fake-input client and the private bus
given only to the daemon (G51's and G52's own fixes, already in place).

Two fixes, both in `sill`:

- `sink.rs`'s `wait_bounded` polls the child with `try_wait` and kills it after
  `COSMIC_RANDR_TIMEOUT` (2 s) instead of `Command::output`'s unbounded wait, so `heads()` fails
  fast on a compositor that never answers and `displays` stays responsive afterward. Tests:
  `a_child_that_answers_is_read_normally`, `a_child_that_never_answers_is_killed_at_the_bound`.
- `daemon.rs`'s `shutdown_runtime` calls `runtime.shutdown_timeout(RUNTIME_SHUTDOWN_BOUND)` (4 s)
  instead of letting `Runtime`'s `Drop` run implicitly, and logs a warning if it took the whole
  bound: any *other* future blocking task that gets stuck the same way leaks its thread rather
  than the process. Tests (`shutdown_tests`): `a_stuck_blocking_task_does_not_hang_the_bound`,
  `a_clean_runtime_shuts_down_promptly`.

`dev/accept-real-input.sh`'s `stop_daemon` no longer relies on `reap`'s kill-after-5-s to end the
daemon (a real hang would now be silently swept away by it): `await_exit` waits for the process
to end on its own and fails the script if it does not. All six steps still pass, twice in a row,
with the daemon and its `cosmic-randr` child both gone (checked by `pgrep`) after each run.

## M6 freeze (2026-09-25)

Notifications (design/20 §1.6, design/13 §13.3.6): an `org.freedesktop.Notifications` server,
banners, and the notification center, frozen on `m6-freeze` so three agents can fill them in
parallel and a fourth can prove them under real input. Everything compiles; every pure function
a later agent fills is `todo!()` with its table test `#[ignore = "M6 fill: <lane>"]`, so the
suite stays green and each agent's first step is to delete its `ignore`s. Findings are numbered
from F260/Q120/G60; the lanes take F280/Q130/G70 (a), F300/Q140/G80 (b), F320/Q150/G90 (c) and
F340/Q160/G100 (d) so they cannot collide.

- **F260 Dependencies: none added.** The server is `zbus` 5 (pinned block, `tokio` feature, no
  defaults): `notifications/dbus.rs` holds the `#[interface(name =
  "org.freedesktop.Notifications")]` object with `Notify`, `CloseNotification`,
  `GetCapabilities`, `GetServerInformation` (`out_args`) and the `NotificationClosed`,
  `ActionInvoked` and `ActivationToken` signals, so the macro and `SignalEmitter` are proven to
  build on the workspace's features before anyone starts. The body markup is parsed by hand (the
  spec's subset is five tags and the XML entities: no `quick-xml`/`html5ever` for that);
  `image-data` is converted to RGBA by hand; the history is `serde_json`, its path `dirs`. Not
  read: cosmic-notifications' source (GPL-3.0, not on this machine); the freedesktop
  Notifications Specification 1.2 is enough, and lane a reads the GPL source, if at all, for
  facts only (CONVENTIONS "Borrowing": note-and-close, never pasted, never into a prompt).
  `cargo deny check licenses` is unchanged.
- **F261 The service's shape.** `sill_services::notifications`: `spawn(rt)` (session bus,
  history in `$XDG_STATE_HOME/sill/notifications.json`) and `spawn_with(rt, Setup { bus:
  BusChoice::{Session, Address}, history: HistoryPlace::{State, At, NotKept} })`, returning two
  receivers and a control. `Notifications { server: ServerStatus, banners: Vec<LiveBanner> }` is
  what the banners follow; `History { groups: Vec<AppGroup> }` (grouped by `AppKey`: the
  `desktop-entry` hint, else `app_name`; groups and notes newest first; no empty group) is what
  the center follows: two channels, as Wi-Fi is apart from `Network`, so a banner coming and
  going does not repaint a closed center. `Services` gains `notifications`,
  `notification_history` and `notifications_control`. A `Notify` becomes a `Posting` once, at
  the boundary (`posting_from_wire`: markup to `Body` runs, actions paired, hints typed:
  `NotifyUrgency`, `category`, `NoteImage::{None, Icon, Pixels}`, `Transience`, `Residency`,
  `SoundHint`, `Expiry::{Default, Never, Ms}`), then a `Notification` with an id and a
  wall-clock `UnixMs` (persisted, so not the monotonic `Stamp`). The one reducer is
  `reduce_notifications(NotesState, NotificationsEvent, &NotificationsPolicy) -> (NotesState,
  Vec<NotificationsEffect>)`: signals (`Closed { id, reason }`, `Invoked { id, action, token }`),
  `SaveHistory` and `Sound` are returned, not performed. Its rules are its module doc and
  `reduce_tests.rs` (12 tables); `assign_id` counts up from the history's `last_id`, reuses a
  held `replaces_id`, and skips 0.
- **F262 Leaving, closing, and where the time is counted.** The banner's hold
  (`notifications.banner_hold_ms`, 5200, paused while hovered: the existing banner machine) is
  counted by the banner surface, which says `BannerTimedOut(id)`; the sender's
  `expire_timeout` only makes a banner persist (0), as design/13 §13.3.6 says. A banner that
  times out **stays in the history and emits nothing** (the notification still exists: a later
  click in the center still invokes its action); a `transient` one, which the history never
  held, is then `NotificationClosed` reason 1. A swipe, the close button, a center row's clear
  and `DismissAll` are **dismissals**: removed from the banners and the history, reason 2
  (design/13 acceptance 10). An action is `ActivationToken` (when the surface got a token) then
  `ActionInvoked`, then, unless `resident`, a dismissal. `CloseNotification` is reason 3. Open
  question for the user: macOS moves a swiped banner into Notification Center rather than
  deleting it; design/13 makes the swipe a dismissal, and the freeze follows design/13.
- **F263 It never fights another daemon.** The name is requested only while the policy says
  `ServerMode::Own`, with `AllowReplacement | DoNotQueue` and never `ReplaceExisting`: a holder
  (cosmic-notifications, mako, plasmashell) keeps it, the service reports it
  (`NameHolder::Other(NameOwner { unique, pid, command })`), and it asks again only when
  `NameOwnerChanged` says the name fell free; a daemon that asks with `ReplaceExisting` later
  takes it (`NameLost`, reported). **Until the daemon's first `NotificationsCommand::Policy`
  the service is passive** (`ServerMode`'s default), so `Services::spawn` in a test or a
  headless render never asks for the name; `notifications.server` (`Own` by default) reaches it
  through `crates/sill/src/notifications/policy.rs::spawn_policy`, which follows the settings
  and the focus link and sends the policy at start and on every change. On this machine
  plasmashell holds the name on the real session bus (`busctl --user status
  org.freedesktop.Notifications`), so a `sill run` in the real session is passive in effect.
- **F264 A private bus that activates nothing.** `dbus-run-session` with the stock
  `session.conf` is not private enough: `/usr/share/dbus-1/services/
  org.kde.plasma.Notifications.service` claims `org.freedesktop.Notifications`
  (`Exec=/usr/bin/plasma_waitforname ...`), so a `Notify` on a fresh bus with nobody holding the
  name asks the bus to activate KDE's. `dev/private-bus.conf` is `session.conf` without service
  directories; `tests/support/private_bus.rs` starts `dbus-daemon --config-file=...` and every
  live test connects by address (`BusChoice::Address`), never through
  `DBUS_SESSION_BUS_ADDRESS`, so `cargo test -- --ignored` cannot reach the real bus. Proven:
  `the_private_bus_starts_empty_and_activates_nothing` passes (a call to the unowned name fails;
  nobody holds it after). The acceptance scripts run the daemon under `dbus-run-session
  --config-file=dev/private-bus.conf`.
- **F265 The banners are one warm column per output, shown on one.** `surfaces/banners/`:
  `Banners { output }`, an Overlay `sill-banners-<out>` anchored `TOP | RIGHT | BOTTOM` (a stack
  of any height fits; `notifications.banner_position` moves the cards inside it), width
  `banner_width_px` plus 40 px of shadow room each side, exclusive zone `Auto` (under the bar's
  reserve, above the dock's), keyboard `None`, input and blur empty until the surface sets them
  to its cards' rounded rects through `shell_host::use_surface()` (`regions::card_regions`),
  because one `RegionSpec::Element` cannot name several cards (G60). Spawned before the OSD, so
  the OSD draws over a banner on the Overlay layer. Banners show only on the output the
  `BannerLink` names, the daemon's first output (macOS shows notifications on the main display;
  one output also means a banner is counted, paused and dismissed once). Frozen pure parts:
  `stack::stack` (grouping by app, the count, `stack_max`, an alert keeps its card persistent),
  `commands::{leave_commands, action_command}`, `regions::card_regions`. The banner machine
  moved from `surfaces/notifications/banner.rs` (design/13 §13.4's path) to
  `surfaces/banners/machine.rs`, unchanged but for `Persistence::of(BannerLife)`.
- **F266 The notification center is a layer, not a popup** (design/20 §1.6 allows either).
  `surfaces/notification_center/`: `NotificationCenter { output }`, an Overlay
  `sill-notification-center-<out>` anchored on all four edges with exclusive zone `Auto` (so the
  compositor keeps it below the bar), keyboard `OnDemand`, input `Whole` (the transparent part
  left of the column is the click-outside catcher, as the power menu's scrim is), blur
  `Element("notification-center-panel")`; the column is `notifications.center_width_px` (384)
  at the right edge and slides in from it. Why: macOS 11 on opens Notification Center from the
  clock as a full-height column pinned to the right edge, not a menu hanging from the item; the
  history and the widgets slot want the output's height; `sill notifications toggle` has no
  pointer serial for a grabbing popup (F179's problem); and a popup would join the bar's hover
  switch, which Notification Center does not. One `CenterLink` (`CenterShown::{Closed,
  Open(output)}`, implemented and tested) is written by the clock and the CLI and followed by
  every output's surface. The bar's clock stops opening its date menu (`BarMenu::Clock`): a
  click toggles the center on its output (lane c). Frozen pure parts: `groups::{group_views,
  toggle_group}` (collapsed to the newest with "N more"), `keys::step_keys` (Escape, Up/Down,
  Return, Delete). `CenterWidgets` is the empty widgets slot (design/20 §1.14, M10). Lane c
  checks on cosmic-comp and KWin that an all-anchored `Auto` layer really sits under the bar's
  reserve; if not, `Ignore` with a top margin of `bar.height_px` is the fallback.
- **F267 One card for both.** `surfaces/notification_card/`: `NotificationCard { note, count,
  expansion, place, now, on_press }` with `CardPress::{Body, Action(key), Close, Link(href)}`,
  `Expansion::{Collapsed, Expanded}`, `CardPlace::{Banner, Center}`, and the labels (`age`,
  `age_label`: now/5m/2h/3d; `count_label`; `app_label`). Lane b owns it and lane c only mounts
  it, so a banner and a center row read alike. It draws nothing until lane b fills it, and it
  is built from quire parts only until quire has a notification card (Q120).
- **F268 Hooks, IPC, CLI.** `use_notifications() -> ReadSignal<Notifications>` and
  `use_notification_history() -> ReadSignal<History>`. `Request::Notifications(
  NotificationsRequest::{Toggle, Open, Close, DismissAll, Status})`, `Status` answered with
  `Response::Notifications(NotificationsReport { mode, holder, banners, kept })` (an IPC mirror
  of `ServerStatus`: `sill-ipc` may not reach `sill-services`); `DebugRequest::Notify(
  FakeNotification { app, summary, body, urgency, actions, expire, replaces })` posts through
  `NotificationsCommand::Post`, the same reducer as `Notify`, without D-Bus. Wire spellings are
  pinned in `sill-ipc/tests/serde.rs`. CLI: `sill notifications
  {toggle,open,close,dismiss-all,status}` and `sill debug notify SUMMARY [--app] [--body]
  [--urgency low|normal|critical] [--action KEY=LABEL]... [--expire default|never] [--replaces
  N]` (`cli_notifications.rs`). Seen on the nested cosmic-comp (its own runtime dir, a private
  bus): `sill debug stats` lists `sill-banners-WINIT-0` and `sill-notification-center-WINIT-0`
  beside the rest; `status` answers `{"mode":"passive","holder":{"kind":"no_bus"},...}` (the
  live loop is a stub); `toggle` and `close` answer done; `debug notify` and `dismiss-all`
  answer "notifications: the service has stopped" until lane a fills the loop.
- **F269 Daemon wiring.** `OutputSurfaces` gains `banners` and `notification_center`;
  `crate::links::Links` gains `notification_center: CenterLink` and `banners: BannerLink`, which
  `surface_root` adds to every root; `crate::notifications` (`spawn_missing`, `banner_output`,
  `dispatch`, `fake`, `report_of`, `namespaces`, `spawn_policy`) is implemented and tested; the
  daemon sets the `BannerLink` after every event's spawns.
- **F270 Settings.** The brief's keys against design/22 §3.12: the banner position is the
  existing `position_right_px` and `position_top_gap_px` plus a new corner,
  `notifications.banner_position`; the most visible banners is the existing `stack_max`; the
  default expiry is the existing `banner_hold_ms` (design/13: "banner duration is not a
  setting", kept Advanced as design/22 has it). New, in `AWAITING_ROWS` (row text in Q126):
  `notifications.server`, `notifications.banner_position`, `notifications.history_cap`,
  `notifications.center_width_px`. Per-app styles and mute are later: design/22's
  `banner_style` row says "per app" but is one value; a table key needs a design/22 shape for a
  map first, so it is proposed in Q126 and not registered.
- **F271 Left out on purpose.** Playing the sound (the sound service is still `inert()`: the
  reducer returns `Sound`, the loop drops it, and `sound` is not advertised); inline reply,
  `action-icons`, `body-images` (an `<img>` shows its alt text), `icon-multi`; the
  `x-canonical-private-synchronous` replace hint; per-app settings (F270); a DND schedule.

### The M6 fill-in: three agents, disjoint files, then real input

Every agent reads CONVENTIONS §0 and §1 (fill bodies, add private helpers and tests; never
change a frozen signature, public field or variant; stop and report instead), ORCHESTRATION
"Coherence rules", design/20 §1.6, design/13 §13.3.6 and §13.8 (acceptance 9-11), and this
section. Everyone runs, before reporting:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features sill/debug
./scripts/check-boundary.sh
cargo deny check licenses
```

(`cargo fmt -p <crate>` while the wave runs.) **No test or script may reach the real session
bus's `org.freedesktop.Notifications`**: live tests use `tests/support/private_bus.rs`, scripts
run the daemon under `dbus-run-session --config-file=dev/private-bus.conf` (F264), and a script
that starts `sill run` gives it its own `XDG_RUNTIME_DIR` (with `WAYLAND_DISPLAY` as an absolute
socket path) when another sill may be running, so latchkey's single-instance lock does not hand
its requests to somebody else's daemon. Files not listed under an agent are read-only to all:
in particular `sill-services/src/{lib.rs,services.rs}`, `sill-surfaces/src/hooks/**`,
`sill-surfaces/tests/support/fake.rs`, `sill-settings/**`, `sill-ipc/**`, and
`crates/sill/src/{daemon.rs,cli.rs,cli_notifications.rs,links.rs,outputs.rs,
notifications/mod.rs,notifications/tests.rs}`.

**(a) a-server: the D-Bus server, the state, the history, Do Not Disturb.**
- Owns: `crates/sill-services/src/notifications/**` (new private files too, e.g.
  `name_watch.rs`), `crates/sill-services/tests/notifications_live.rs`,
  `crates/sill-services/tests/support/**`, `crates/sill-services/Cargo.toml` (an `[[example]]`
  entry only, e.g. a name holder for the "never replaced" check), new
  `crates/sill-services/examples/*.rs`, `crates/sill/src/notifications/{fake.rs,policy.rs}`,
  `dev/private-bus.conf`, a new `dev/accept-notifications-bus.sh`.
- Frozen: every `pub` item `sill_services::notifications` re-exports (types, fields, variants,
  `serde` attributes), `spawn`, `spawn_with`, `Setup`, `BusChoice`, `HistoryPlace`,
  `NotificationsHandles`, `NotificationsCommand`, `BUS_NAME`, `OBJECT_PATH`, the signatures of
  `parse_body`, `presented`, `chime`, `assign_id`, `reduce_notifications`, `pair_actions`, the
  `history_file` API and `STORED_VERSION`, `posting_of`, `policy_of`, `spawn_policy`. `dbus.rs`
  is private: its D-Bus names and signatures are the spec's; its Rust shape is a's.
  `CAPABILITIES` may gain `sound` only when a hint really plays.
- Done: every `M6 fill: a-server` test passes un-ignored (`markup`, `policy`, `reduce_tests`,
  `dbus`, `notifications_live`); `live::run` connects to `setup.bus`, publishes `NameHolder`
  (asking `GetConnectionUnixProcessID` and reading `/proc/<pid>/comm` for another holder),
  serves the object and requests the name only under `Own` (F263), follows `NameOwnerChanged`
  and `NameLost`, loads the history (`HistoryPlace`), runs `Call`s and commands through
  `assign_id` and `reduce_notifications`, emits the signals, saves after `SaveHistory`, drops
  `Sound`, and ends when every control is dropped; `CloseNotification` of an id no longer held
  answers as spec 1.2 says (an empty D-Bus error); `sill debug notify` parses its body with
  `parse_body` (`fake.rs`).
- Verify: the gates; `cargo test -p sill-services --test notifications_live -- --ignored
  --nocapture` (four tests); `dev/accept-notifications-bus.sh` on the nested compositor, the
  daemon under the private bus with a scratch config: (1) `sill notifications status` says
  holder `sill`, mode `own`; (2) `gdbus call ... GetServerInformation` answers `('sill', 'sill',
  <version>, '1.2')` and `GetCapabilities` the six words; (3) `notify-send -a accept "Hello"`
  makes `status` count 1 banner and 1 kept; (4) `gdbus call ... CloseNotification <id>` and a
  `gdbus monitor` beside it records `NotificationClosed (<id>, 3)`; (5) with
  `notifications.server = "passive"` and `sill reload` the name is released (status holder
  `nobody`) and nothing answers `Notify`; (6) with a holder started first (the example), mode
  `own` reports holder `other` with that pid and the holder keeps the name; (7) the history
  file under the scratch `XDG_STATE_HOME` survives a daemon restart (`status` kept is the same).

**(b) b-banners: the banner column and the card.**
- Owns: `crates/sill-surfaces/src/surfaces/banners/**` (the implemented `machine.rs`,
  `layer.rs` and `placement.rs` may change inside, never in signature),
  `crates/sill-surfaces/src/surfaces/notification_card/**`, new
  `crates/sill-surfaces/src/style/{banners,notification_card}.css`, new
  `crates/sill-surfaces/tests/{banners_render,banners_lint,notification_card_render}.rs` and
  `tests/banners_support/**`, a new `dev/accept-notifications.sh`.
- Frozen: `Banners { output }`, `banners_layer`, `ROOM`, `CARD_RADIUS`, the machine's public
  API, `Card`, `stack`, `leave_commands`, `action_command`, `CardBox`, `card_regions`,
  `BannerLink`, `BannerOutput`; `NotificationCard`'s props, `CardPress`, `Expansion`,
  `CardPlace`, `Age`, `age`, `age_label`, `count_label`, `app_label`.
- Done: the ignored tests pass; the column follows `use_notifications` and the `BannerLink`,
  maps itself (`set_visibility`) while its output shows a card and unmaps once the last card's
  exit has settled; `stack` picks the cards; one `machine::step` per card, woken with quire's
  timers only (`ds::sleep`, `settle`), `Persistence::of(card.life)`; hovering pauses and expands
  (six lines, `Button{Mini}` actions, the 18 px close button at the top-left), a click on the
  body sends `action_command(default)` with the surface's activation token
  (`use_surface().activation_token`), a button its action, the close button and a swipe or drag
  past the threshold `leave_commands(Dismissed)`, a hold's end `leave_commands(Timeout)`; the
  input and blur region follow `card_regions`; the card's entrance, exit and heal are quire
  motion only (Q121, Q122: until quire has them, the cards appear and leave without a keyframe
  of sill's own, and the gap says so); `banner_position` and `banner_entry_direction` honoured;
  only quire components (the SSR class test and `ds::lint::stylesheet`, as the OSD's tests do).
- Verify: the gates; `dev/accept-notifications.sh` on the nested compositor (daemon under the
  private bus, scratch config, `--features debug`, every notification from `sill debug notify`):
  (1) `sill-banners-<out>` exists and is hidden at start (capture fails); (2) a notification
  shows it within 300 ms (capture with card pixels), `dev/out/banners-cosmic.png`; (3) it hides
  between the hold (5200) and the hold plus the exit plus 400 ms, and `status` then says
  banners 0, kept 1; (4) hovered at 4000 ms (`sill debug pointer` on the card) it is still up at
  7000, and gone by 1500 ms plus the exit after `sill debug leave`; (5) three from one app
  within 2 s make one card with a count of 3, five from five apps three cards
  (`dev/out/banners-stack-cosmic.png`); (6) `notifications.dnd = "on"` and `sill reload`: a
  normal one shows no banner but `status` kept rises, a `--urgency critical` one shows and is
  still up after 10 s; (7) `sill debug click` on a card's body with `--action default=Open`:
  the card leaves and kept falls by one; (8) 5 s after the last card leaves, 0 frames on the
  column. The drag and the swipe are lane d's (debug injection has no drag).

**(c) c-center: the notification center and the clock.**
- Owns: `crates/sill-surfaces/src/surfaces/notification_center/**` (the implemented `layer.rs`
  and `link.rs` may change inside, never in signature); in the bar, `surfaces/bar/clock.rs`,
  and `surfaces/bar/{menus,content,popup,menu_popup}.rs` only to retire `BarMenu::Clock`; new
  `crates/sill-surfaces/src/style/notification_center.css`; new
  `crates/sill-surfaces/tests/{notification_center_render,notification_center_lint,
  notification_center_keys}.rs` and `tests/center_support/**`; `examples/bar_demo.rs` (the
  clock's context); a new `dev/accept-notification-center.sh`.
- Frozen: `NotificationCenter { output }`, `center_layer`, `panel_region`, `PANEL_ID`,
  `PANEL_RADIUS`, `CenterLink`, `CenterShown`, `Expanded`, `GroupView`, `group_views`,
  `toggle_group`, `CenterKey`, `RowFocus`, `CenterOut`, `step_keys`, `CenterWidgets`;
  `NotificationCard` is b's (mount it, never edit it).
- Done: the ignored tests pass; the surface follows the `CenterLink` onto its visibility; a
  `Ds` root (`Popover`) holds the column at the right edge, `center_width_px` wide, entering
  from the right with quire motion only (Q123); a header with Clear All (`Button{Quiet}`); the
  history from `use_notification_history` through `group_views`, each group a header (the
  app's icon and name, Clear) over `NotificationCard { place: Center }` rows, "N more" opening
  the group (`toggle_group`); an empty state; a footer line when another daemon holds the name
  (`use_notifications().server`: "Notifications are shown by plasmashell") or the server is
  passive; `CenterWidgets` below; a row's body invokes `default` and closes the center, its
  close dismisses it, a group's Clear sends `ClearHistory(Clear::App)`, Clear All
  `ClearHistory(Clear::All)`; keys through `step_keys`; a press outside the column closes it;
  the clock toggles the center on its own output (`try_use_context::<CenterLink>()`) and no
  longer opens a date menu.
- Verify: the gates; `dev/accept-notification-center.sh` on the nested compositor (daemon as in
  b): (1) `sill-notification-center-<out>` exists hidden; (2) `sill notifications toggle` shows
  it (capture), `toggle` again hides it, `open` twice stays open, `close` hides; (3) `sill debug
  click` on the bar's clock opens it, a second click closes it; (4) with three notifications
  from two apps (`sill debug notify`), a capture shows two groups,
  `dev/out/notification-center-cosmic.png`; a click on "1 more" opens the group
  (`dev/out/notification-center-open-cosmic.png`); a group's Clear drops `status` kept by that
  group's count; (5) a click left of the column closes it; (6) 0 frames while hidden.

**(d) d-real-input: after a, b and c are merged.**
- Owns: a new `dev/accept-notifications-real-input.sh` and its entries in
  `docs/manual-checks.md`. No source file: a bug found is reported against its lane's files.
- Done and verify: on shell-host's headless KWin (`scripts/dev/kwin-virtual.sh start`,
  `fake-input.sh`, one fake-input client for the whole run: G50), the daemon on KWin's private
  bus with `notifications.server = "own"`, checked to hold the name before anything is posted
  (G61): (1) `notify-send` shows a banner; a real hover at 4 s keeps it past 5.2 s and it leaves
  1.5 s after the pointer does; (2) a real click on the body of one sent with a `default`
  action: `gdbus monitor` records `ActionInvoked (<id>, 'default')` then `NotificationClosed
  (<id>, 2)`; (3) a real drag 90 px right: `NotificationClosed (<id>, 2)`; 50 px slowly: it
  springs back and stays; (4) a real click on the clock opens the center, real Escape closes
  it, a click outside closes it; (5) `sill notifications status` before and after says holder
  `sill`. KWin is stopped at the end, the daemon given the bus only (G52).

### Gaps found in the M6 freeze

- **Q120 (quire), closed** (quire's notification parts, adopted in F360-F362) A notification card (design/13 §13.3.6 "Banner", "Content"): the app icon
  (08 plate, 32 in a banner, 20 in a center row), the app name and the age on one line (time
  data 10 at the right), the summary (ui 13/700) and the body (ui 13) clamped to two lines, six
  when expanded, a count chip with the two offset layers under a grouped card
  (`group_offset_px`), and when expanded the actions as `Button{Mini}` and an 18 px close button
  at the top-left corner; `Material::Toast` painted on the card (or ink on paper per
  `notifications.banner_material`), a center variant on the Popover. `Toast` is mail's one undo
  pill at the bottom edge and `ListRow` is the mail row; neither has this shape.
- **Q121 (quire), closed** (quire's notification parts, adopted in F360-F362) A banner stack host: several cards, newest on top, gap `stack_gap_px`, at
  most `stack_max`; each enters with `slide-l` from `translateX(calc(100% + 8px))` at `--t-big
  --e-spring` (from below with `BannerEntry::FromBelow`), leaves sliding right at `--t-move
  --e-exit` with an `on_hidden` after `settle`, and the cards below heal (`heal`, 18 ms per
  row). `ToastHost` shows one toast and owns its own hub.
- **Q122 (quire), closed** (quire's notification parts, adopted in F360-F362) Swipe-to-dismiss motion: a card that follows a horizontal drag 1:1 to the
  right and damped to the left (transitions off while dragging), springs back at `--t-move
  --e-spring`, or flies out right on a dismiss, and sums a horizontal scroll gesture (Magic
  Mouse, touchpad) to the same thresholds. The numbers and the decision are sill's machine; the
  motion is quire's (rule 4). `DragTracker` is drag-and-drop with a ghost and targets, and the
  toast's `PullTab` is a 46 px undo arm.
- **Q123 (quire), closed** (quire's notification parts, adopted in F360-F362) An edge panel: a full-height column at the output's right edge that enters
  with `slide-l` and leaves to the right, on the Popover material, with a transparent
  click-outside area. `SheetPlacement` has `Top` and `Centre` only.
- **Q124 (quire), closed** (quire's notification parts, adopted in F360-F362) Rich text runs: `Text`'s runs carry tones (strong, faint, a search mark), not
  italic, underline or a link. A notification body needs bold, italic, underline and a link run
  that calls back with its href.
- **Q125 (quire), closed** (quire's notification parts, adopted in F360-F362) A group header with a leading app icon: `SectionHeader` has text, a value and
  one action (enough for "Clear"), but no leading glyph or plate.
- **Q126 (design/22)** The rows `AWAITING_ROWS` exempts (F270), for §3.12:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `notifications.server` | `NotificationServer::{Own,Passive}` | `Own` | asks for `org.freedesktop.Notifications` without replacing a holder; `Passive` leaves it to another daemon | `20-SURFACES.md#1-6-notifications-toasts-and-notification-center-spec-tier-1`; sill FINDINGS F263 | proposed (2026-09-25) |
  | `notifications.banner_position` | `BannerPosition::{TopRight,BottomRight}` | `TopRight` | the corner banners stack in; `BottomRight` sits above the dock's reserve, newest at the bottom | `20-SURFACES.md#3-open-decisions` item 2; `13-BEHAVIOUR-menus-windows.md#13-3-6-notifications` | proposed (2026-09-25) |
  | `notifications.history_cap` | `Count` | `100` | `0..=1000`; the oldest kept notification goes first | sill FINDINGS F261 | proposed (2026-09-25) |
  | `notifications.center_width_px` | `Px` | `384` | `280..=600`; the notification center's column (a banner's 360 plus 12 each side) | `20-SURFACES.md#1-6-notifications-toasts-and-notification-center-spec-tier-1`; sill FINDINGS F266 | proposed (2026-09-25) |

  All Advanced. Later, not registered: `notifications.apps`, a table keyed by app, each entry
  with `style: BannerStyle` and `sound: On|Off` (the per-app form design/22's `banner_style`
  row already names); it wants §4's shape for a map key first.
- **G60 (shell-host)** A region of several elements: `RegionSpec::Element` names one id, and
  design/20 §1.6 wants blur and input per banner card. The banners set `RegionSpec::Rects` from
  their measured cards through `SurfaceHandle::set_input_region`/`set_blur`, which works but
  lags a layout; `RegionSpec::Elements(Vec<(ElementId, radius)>)` (or a class) resolved after
  layout, as `Element` is, would follow the cards exactly.
- **G61 (kwin-virtual.sh)** Its private bus is `dbus-daemon --session`, the stock config with
  service directories, so a `Notify` sent there while nobody holds the name asks for KDE's
  notifications to be activated (F264). A `--config-file` without service directories (sill's
  `dev/private-bus.conf`) would make it as private as it is meant to be; until then lane d
  checks that sill holds the name before posting anything.

## M6 notification center (lane c, 2026-09-25)

Lane c of "The M6 fill-in": `surfaces/notification_center/**`, the bar's clock, and
`dev/accept-notification-center.sh`. Findings take F320-, Q150-, G90-.

- **F320 The pure parts.** `group_views`, `toggle_group` and `step_keys` are filled and their
  tables un-ignored. New pure cores, each with table tests: `phase.rs` (`step_phase`: `Hidden`,
  `Up`, `Leaving`, keyed by an `Opening`; the frozen `CenterLink` has no "leaving" state, so the
  surface counts its own exit and unmaps only when it has settled; wanted again while leaving,
  it comes back without a second entrance); `rows.rs` (`rows_of` flattens the groups into the
  rows the keys move over: cards and fold lines; `activate`, `dismiss` and `press` say what
  Return, Delete and a card's `CardPress` ask); `keymap.rs` (Escape, Up, Down, Enter, and Delete
  or Backspace, the Mac keyboard's Delete); `labels.rs` (the group's name, the fold line, the
  footer).
- **F321 macOS's stack.** A collapsed group of several is its newest card with lane b's count
  chip and offset layers (`count: Stack::count`), then "N more" (`Button{Quiet}`); an opened
  group ends in "Show less". As on macOS, a click (or Return) on a stack opens the group rather
  than invoking the newest notification's `default`, and a stack's close (or Delete) clears the
  whole group (`Clear::App`); a lone card's body invokes `default` with an activation token
  when the compositor gives one (`SurfaceHandle::activation_token`, the sender's desktop entry)
  and closes the center; its close dismisses it. A link in a body asks nothing of the center:
  sill has no URL opener service yet (`rows::press`, `CardPress::Link` is `None`). A row under
  the pointer or the keyboard's ring is drawn `Expansion::Expanded`, as a hovered banner is.
  Each opening remounts the column (keyed by its `Opening`), so opened groups, the ring and the
  hover start over.
- **F322 The group's name is the center's own label.** `labels::group_title` names a group by
  its desktop entry's `Name` (the apps index), else the newest sender's `app_name`, else
  "Notification": lane b's `app_label` says the same for one card, and was a `todo!()` while
  both lanes worked, so the center could not call it and stay green. Once both are merged the
  header can call `app_label(&group.notes[0].app, entry)` instead; nothing else changes.
- **F323 The column is quire's Popover on a stand-in edge panel (Q123).** The root is a
  transparent `Ds { material: Popover, extent: Viewport }` (the whole surface is the click
  catcher: `div.sill-nc-catcher`, a press closes); the column is `div#notification-center-panel`
  (the blur element, `PANEL_ID`) holding a quire `Surface { material: Popover }`, which paints
  the tint, the 14 px corner and the drop, in a one-cell grid so it stretches to the full height
  (`.ds` itself may not be styled: `Rule::DsInternals`). It stands 8 px inside the right, top and
  bottom edges of the surface, `notifications.center_width_px` wide (inline). Motion is quire's
  pulse classes: the entrance `Anim::PaneInR` (`slide-r` at `--t-move --e-spring`, 26 px and a
  fade), the exit `Anim::PaneOutR` (`pane-out-r` at `--t-move --e-exit`), timed by
  `use_motion_timer(PaneOutR)` before the surface unmaps. Q123 still stands: a real edge panel
  would slide the whole column in from past the edge and out again, not 26 px; the spots are
  `column.rs`, `drive.rs` (`ENTRANCE`, `EXIT`) and `style/notification_center.css`.
- **F324 The group header is `SectionHeader{Group}` (Q125).** The app's name, the count as the
  header's value when more than one, and Clear as its one action. No app icon: quire's header
  has no leading glyph or plate, and a local icon beside it would be a component of our own.
  The spot is `group.rs`.
- **F325 The clock.** `ClockItem` no longer sits in a `TitleSlot`: a `MenuBarItem` (its pill on
  while the center is open on this output) around the clock's `Button{Quiet}`, whose click
  toggles the `CenterLink` from `try_use_context` (a bar with no link, as a render test, toggles
  a link of its own; `examples/bar_demo.rs` provides one). `BarMenu::Clock` is retired but not
  removed: nothing opens it now, but `plan.rs`'s tables name it as a stand-in status menu and
  `bar/mod.rs` still builds `BarState.clock`, both outside lane c; `content.rs` still answers
  its date lines if asked. Removing the variant is a follow-up for whoever owns those files.
  `dev/accept-bar.sh` and `dev/accept-bar-ui.sh` click the clock expecting a menu popup; they
  now open the center, and need another item for their menu step (not lane c's files).
- **F326 Under the bar on cosmic-comp.** The all-anchored Overlay with exclusive zone `Auto` is
  kept below the bar's reserve and above the dock's by cosmic-comp: at 2560 x 1600 the center's
  surface is 2560 x 1500 (the bar's 32 and the dock's reserve outside it), the column at
  2168..2552 x 8..1492 in it. The `Ignore`-with-a-margin fallback of F266 is not needed there;
  KWin is lane d's to see.
- **F327 Joining lane b's card.** The card's stylesheet is `notification_card::CARD_CSS`
  (lane b); the center's root must inject it next to its own: in `panel.rs`,
  `style { {CARD_CSS} }` before `style { {NOTIFICATION_CENTER_CSS} }`, and the lint test's
  `full_css` gains it. It cannot be written on `m6-c` before `m6-b` is merged. Proven on a
  scratch merge of m6-a, m6-b and m6-c: without it the cards draw unlaid-out; with it they read
  as the banners do (`dev/out/notification-center-cosmic.png`).

### Acceptance (nested cosmic-comp, 2560 x 1600, on a scratch merge of m6-a + m6-b + m6-c)

`dev/accept-notification-center.sh`, the daemon under `dbus-run-session
--config-file=dev/private-bus.conf` with its own `XDG_RUNTIME_DIR`, scratch config and state:

```
PASS step_warm        sill-notification-center-WINIT-0 spawned at start, hidden
PASS step_toggle_open_close  toggle shows the column (box 2168 8 2552 1492 in a 2560 1500
                      surface) and hides it; open twice stays open; close hides it
PASS step_clock       a click on the clock at 2494 16 opens the center; a second closes it
PASS step_groups      kept 3; '1 more' at 2359 197 opened Mail's group; Mail's Clear: kept 1
PASS step_outside     a click left of the column closes it
PASS step_idle        5 s idle open: 0 frames; 5 s hidden: 0 frames
```

On `m6-c` after merging master (lane a in, lane b not yet) the same six steps pass too
("kept 3; '1 more' at 2360 106 opened Mail's group; Mail's Clear: kept 1"; the cards are drawn
unlaid-out until F327's line lands); before lane a, step 4 is a SKIP (exit 2): `sill debug
notify` answered "the service has stopped". Screenshots, from the scratch merge:
`dev/out/notification-center-cosmic.png` (two groups, Mail's stacked with "1 more"),
`dev/out/notification-center-open-cosmic.png` (Mail's opened, "Show less"); and from m6-c,
`dev/out/notification-center-empty-cosmic.png` (the empty state). Keys are not driven live
(the nested compositor gives the layer no keyboard, as it gave the power menu none; shell-host
injects no keys): the harness proves them
(`tests/notification_center_keys.rs`), real Escape is lane d's.

### Gaps found in the notification center

- **Q123 (quire), closed** (quire `Panel`; "Adopting quire's notification parts" F361): was the stand-in of F323 (`column.rs`, `drive.rs`, the stylesheet).
- **Q125 (quire), closed** (quire `GroupHeader`; F361): was the stand-in of F324 (`group.rs`).

## M6 lane a: the notifications server (2026-09-26)

- F280 Frozen additions, all additive: `NotificationsPolicy.swipe: Swipe::{KeepInCenter, Dismiss}`
  (from `notifications.swipe`), `NotificationsCommand::SwipeAway(id)` and
  `NotificationsEvent::SwipedAway(id)`. Lane b sends `SwipeAway` for a swipe past the threshold
  (each banner of a group) and keeps `Dismiss` for the close button.
- F281 A kept swipe sends no `NotificationClosed`, like a banner that times out (F262): the
  notification is still live in the notification center and its actions still work there. The
  app hears `Closed` only when it truly leaves (close button, Clear, `CloseNotification`), or
  when a transient one (never in the history) is swiped (reason 2).
- F282 `examples/notifications_holder.rs` holds the name on a private bus so the acceptance can
  prove sill never replaces a holder; `dev/accept-notifications-bus.sh` passes 7/7 on its own
  bus, and the real session bus keeps plasmashell as holder throughout.

## M6 lane b: the banners and the notification card (2026-09-26)

- **F300 The column is a deck.** `surfaces/banners/deck.rs` (pure, `deck_tests.rs`) holds the
  drawn cards: the `stack`'s plus the ones still sliding out, one banner machine each, their
  pose (`Offstage`, `Rest`, `Follow(dx)`) and what the column must do next (`Rise`, `Entrance`,
  `Exit`, `Send`). `drive.rs` owns the timers (`ds::sleep` for `FRAME_SLACK`, `DurationToken::Big`
  and `Move` at the root's motion level, and the hold: no literal duration), `gesture.rs` (pure)
  turns a press into a click or a drag (4 px slop; the release velocity is measured from the move
  before the last one, so a drag that pauses springs back and a flick flies out), `look.rs` (pure)
  maps the pose onto `data-pose` and `data-transit`, and `view.rs` draws. A newer notification
  from a drawn app replaces the card's content and restarts its hold; a card the service dropped
  (CloseNotification, Dismiss All) slides out without a word to the service; a stack card the
  leaving card already stood for is not a new card.
- **F301 The hold starts as the card rises**, not when its entrance has settled: design/13
  acceptance 9 and the freeze's step 3 time the 5200 ms from the notification, and a 420 ms
  entrance before it would put the hide past hold + exit + 400. The entrance keeps its own
  transition (`Arrival::Rising`) until it has played.
- **F302 Swipe vs close.** The machine gained `Leave::Swiped` (a drag or a horizontal swipe past
  the threshold) beside `Leave::Dismissed` (the close button, CloseNotification, a
  replacement); `leave_commands` sends `SwipeAway` for the first and `Dismiss` for the second,
  once per grouped banner (F280: under the default `notifications.swipe = "keep_in_center"` the
  service keeps a swiped notification in the center, as macOS does).
- **F303 A click without a `default` action** dismisses the card (macOS removes a clicked
  banner); with one, it sends `InvokeAction(default)` with the surface's activation token (for
  the app's desktop entry) and the card leaves as acted.
- **F304 Regions are element regions.** Each card writes ids on its painted parts
  (`notification_card::part_id`: `note-<id>-face|near|far|close`); the column's input is those
  parts (a press between cards falls through), its blur the face and the group layers, both as
  shell-host's `RegionSpec::Elements` (G60 closed upstream), and the whole surface while an
  actions menu is open. `regions::card_regions` (the freeze's measured-rects form) stays for its
  frozen signature but is no longer drawn from.
- **F305 The card** (`notification_card/`): a quire `Surface` in `Material::Toast` (ink on paper
  under `banner_material = "inverse"`: the same material in the other scheme; `Popover` for a
  center row) with the app icon (the sender's `app_icon`, a theme name through the apps
  service's lookup, else its desktop entry's icon found by `match_app` or `Name`, else quire's
  bell), the app name (`app_label`, which the center's group title now uses too) and the age,
  the summary, the body clipped to two lines (six expanded) with `ds::clip_chars` and an `em`
  max-height, `Count` for a group and two sliver layers under it (translucent materials show a
  whole card behind), and, expanded, up to two actions as `Button{Mini}` or more behind an
  `Options` button opening a quire `Menu`, and the close button (`IconButton{Strip}` on a round
  `Surface`) over the top-left corner. Settings lengths reach the sheet as custom properties on
  the slot (`CARD_VARS`, declared to the lint).
- **F306 Acceptance** (`dev/accept-notifications.sh`, nested cosmic-comp, private bus and
  runtime dir, `sill debug notify` only): 8/8. Shown 208 ms after the notification; hidden at
  5719 ms (limit 5850), then banners 0, kept 1; hovered at 4000 ms still up at 7000, gone 1870 ms
  after the pointer left (limit 2150); three from one app draw one card 131 px tall with status
  banners 3, five apps three cards (250 px); under DND a normal one is kept without a banner and
  a critical one is still up after 10 s; a click on a `default=Open` card leaves in 292 ms and
  kept falls 4 -> 3; 0 frames 5 s after the last card. Screenshots `dev/out/banners-cosmic.png`
  and `dev/out/banners-stack-cosmic.png`. `dev/accept-notification-center.sh` also passes 6/6
  with the card's stylesheet in the center, `dev/out/notification-center-cosmic.png` retaken.

### Waiting on quire (lane b's stand-ins; all closed by "Adopting quire's notification parts", F360-F362)

- **Q120** the card itself: `notification_card/view.rs` composes `Surface`, `IconView`/`Glyph`,
  `Count`, `Button{Mini}`, `Menu` and `IconButton{Strip}` in sill's layout sheet
  (`style/notification_card.css`). Also waiting: the 08 plate under the icon, an 18 px close
  button (`close_button_px`; quire's Strip is 26 px, so `elements.rs` rounds it by 13), the line
  clamp (`body.rs` counts characters), and the height animating on expand.
- **Q121** the stack host: entrance, exit and spring are sill's transitions on quire tokens
  (`style/banners.css`, `look.rs`), and the heal of the cards below a leaving one is missing (they
  close the gap at once).
- **Q122** swipe motion: the follow and fly-out are the same transitions; the horizontal scroll
  gesture (Magic Mouse, touchpad) is not summed yet (Blitz's wheel events carry no phase; the
  machine's `BannerIn::Swipe` is ready for it).
- **Q124** rich runs: the body is drawn as plain text (`Body::text`), so bold, italic, underline
  and links (`CardPress::Link`) wait.
- **Q125** is lane c's (the group header); the center shows the card's `app_label`.

## M6 lane d: real input, and lane c's clean-ups (2026-09-26)

Lane d of "The M6 fill-in": `dev/accept-notifications-real-input.sh` on shell-host's headless
KWin, then F325's follow-ups. Findings take F340-, Q160-, G100-.

- **F340 The real-input acceptance.** The daemon runs from the debug build on the headless KWin
  (`kwin-virtual.sh start 1920x1080`) and its private bus, which since shell-host F56 is
  `private-bus.conf` (G61 closed upstream), with its own `XDG_RUNTIME_DIR` (`WAYLAND_DISPLAY` the
  absolute socket), scratch config and state, `notifications.server = "own"`; the script reads
  `.kwin-env` without sourcing it, so `kwin-virtual.sh stop` never meets it. Every notification is
  `sill debug notify`; one fake-input client serves the run; a `gdbus monitor` on KWin's bus
  records the service's signals. The column is found from KWin's window list
  (`kwin-windows.sh`: the 440 px wide sill window, at 1512,0 on KWin: its negative margins put it
  32 px past the right edge and over the bar's reserve, as on cosmic-comp) and the card from a
  capture of it (1552..1912 x 40..122 for a two-line card).
- **F341 KWin keeps the center under the bar too.** The all-anchored Overlay with exclusive zone
  `Auto` is `[0, 32, 1920, 980]` on KWin: below the bar's 32 and above the dock's 68 reserve, as
  F326 found on cosmic-comp. It takes the keyboard when opened by a click on the clock (KWin marks
  it active), so real Escape, Down and Delete reach it.
- **F342 `BarMenu::Clock` and `BarState.clock` are gone** (F325's follow-up). The menu-tracking
  tables in `plan.rs` used the clock as their stand-in status menu; they now use the Volume item
  (a status menu on the same side; each case opens one menu), so no case was lost. `BarState`
  no longer reads `use_clock`, so the bar's root no longer re-renders on the minute: only
  `ClockItem`, which reads it itself, does. `content.rs`'s app menu still uses `lines_menu`.
- **F343 The bar scripts' menu step.** Both `dev/accept-bar.sh` and `dev/accept-bar-ui.sh` now
  open the control center's item (the status item left of the clock, found from a capture of the
  bar as `dev/accept-real-input.sh` finds it), keeping their intent (a title opens the bar's one
  popup, an outside press dismisses it). `accept-bar.sh` gains `step_clock_center`: a click on the
  clock opens `sill-notification-center-<out>` and no bar popup, a second closes it.
  `accept-bar-ui.sh`'s transparency check allows alpha 16 instead of 8: the control center hangs
  8 px under the popup's top edge, so the middle of that edge is in its drop shadow (alpha 10);
  the date menu had more room.

### Acceptance

`dev/accept-notifications-real-input.sh` (headless KWin 1920 x 1080, real pointer and keys):

```
PASS step_holder              status: holder sill, mode own
PASS step_hover               hovered at 4000 ms: still up at 7000 ms; gone 1718 ms after the
                              pointer left (limit 2150); kept 0 -> 1
PASS step_close_button        hovered, the close button shows; a real click on it at 1556,44:
                              gone in 181 ms, kept 2 -> 1
PASS step_click_default       a real click on the body: the card left in 139 ms, kept 2 -> 1;
                              the bus: ActionInvoked (uint32 3, 'default')
PASS step_swipe               a real 90 px drag right: gone in 131 ms; banners 0, kept 2
                              (kept in the center)
PASS step_spring_back         a real 50 px drag, slowed: the card is back at x 1552, banners 1
PASS step_center_clock_escape a real click on the clock (1852 16) opened the center; real Escape
                              closed it
PASS step_center_outside      a real click outside the column (200 540) closed the center
PASS step_center_delete       in the center, real Down then Delete cleared chat's group:
                              kept 4 -> 2
PASS step_idle                5 s idle: 0 frames on the banners and on the center
PASS step_holder              status: holder sill, mode own
```

KWin, the input client, the monitor and the daemon (`sill quit`) are stopped at the end.
`dev/accept-bar.sh` on the nested cosmic-comp (2560 x 1600; daemon under `dbus-run-session
--config-file=dev/private-bus.conf`, scratch config): 9/9, `step_menus` "opened: yes, dismissed:
yes" (the control center at 2418,16), `step_clock_center` "the clock opened the center: yes, a
bar menu: no, a second click closed it: yes"; `dev/out/bar-cosmic-menu.png` retaken with the
control center open. `dev/accept-bar-ui.sh` (its own headless KWin): exclusive zone, idle,
menus and popup transparency pass (corner alphas 1 1 0 0 10 8); `dev/out/bar.png` retaken.

### Gaps found under real input (shell-host; `dev/accept-notifications-real-input.sh repro`; both closed by shell-host 902a12c, F364)

Neither is sill's to fix; both FAIL in `repro` mode today, and the acceptance is ordered around
them (a move is a move and a 1 px nudge; the close button comes before the first click).

- **G100 (shell-host) An enter moves nothing.** `handlers/pointer.rs` takes
  `PointerEventKind::Enter` (focus, zone, cursor) but does not move the document's pointer to its
  position, so a pointer that reaches a surface in one motion (KWin sends the enter with the
  position and no motion) hovers nothing until it moves again. Repro step
  `repro_enter_one_motion`: "one motion onto the card: hovered no; after a second motion: yes".
  Real mice send streams of motions, so this mostly hides; a banner that appears under a resting
  pointer and one small nudge show it. Fix: `pointer_moved(id, pos)` on enter.
- **G101 (shell-host) An unmapped surface keeps its hover.** After a card leaves under the pointer
  (a click on it, or its close button) the column unmaps; `ctx/ops.rs::unmap` rebuilds the
  wl_surface without `document.pointer_left`, and the compositor's leave then names a surface
  the host no longer knows (`surface_id_for` is `None`), so the document keeps a hover chain on
  the removed card. The next card is never announced as entered: no `mouseenter`, so **no hover
  pause and no close button**, even when the pointer went elsewhere while the column was unmapped
  and comes back with a move and a nudge (most likely the new card's node reuses the removed
  one's id, so Blitz sees no hover change). Leaving the column and re-entering it hovers it.
  Repro step `repro_hover_after_unmap`: "a card mapped under the pointer, moved on: hovered no;
  out and back: yes". Fix: clear the document's pointer (`pointer_left`, and `pointer_focus` if
  it named the surface) in `unmap`. User-visible: after clicking one banner, the next one may
  time out under the pointer.

## Adopting quire's notification parts (2026-09-26)

Branch `adopt-notif-parts`: the banners and the notification center move onto quire's
notification parts (quire CONSUMING.md "Notification parts", FINDINGS "Notification parts"),
closing Q120-Q125. Findings take F360-, Q170-.

- **F360 The banners.** One transparent Toast root (`Ds { material: Toast, chrome:
  Some(RootChrome::Transparent), extent: Viewport }`), `NotificationMetrics` (the eight
  `notifications.*` geometry keys, `notification_card::notification_metrics`) written on the
  `.sill-banners` wrapper, holding quire's `BannerStack { banners, position, on_hidden }`. Each
  banner is a `ds::Banner { key: BannerKey(entry), card }` whose card is quire's
  `NotificationCard` (through sill's `NoteCard`): `AppMark` (the icon as before, the name by
  `app_label`), the age, the summary, the body as `Rich` (F362), `GroupCount { count, layers:
  Layers(min(count - 1, 2)) }`, every action but `default` as a `CardAction`, `id:
  "toast-<id>"`, `swipe: Swipe::Dismiss`, `swipe_metrics` from the swipe keys, `icon_size` from
  `icon_px`, `on_hover` pausing and resuming sill's hold. The banner key is the deck's entry
  (`EntryKey(u32)`, one per arrival), not the notification id: an app's card keeps its row while
  newer notifications replace its content. sill keeps the hold machine (`machine.rs`, now
  without drag state: quire decides the swipe and the machine hears `BannerIn::Swiped`), the
  stack and grouping (`stack.rs`), the deck (`deck.rs`, which lists every card not leaving and
  keeps a leaving one until the stack's `on_hidden` feeds `BannerIn::Left`) and the commands:
  `on_dismiss` sends `SwipeAway` (the service honours `notifications.swipe = KeepInCenter`), the
  close button `Dismiss`, a press `InvokeAction(default)` or, with no `default`, `Dismiss`. The
  surface unmaps when `on_hidden` has emptied the deck. Input and blur stay
  `RegionSpec::Elements`: the plate `toast-<id>` (input inflated by 6 and rounded by 9 so quire's
  18 px close button, standing 6 px out past the corner, takes its whole press; blur rounded by
  16), and for a group the whole card `banner-<id>` whose layers reach below the plate. Deleted:
  `style/notification_card.css`, `banners.css`'s slide and stack rules (it now only places the
  stack), `look.rs`, `gesture.rs`, the card's hover expand, close button, `Options` menu and
  character clamp (`Expansion`, `MenuOpen`, `MenuSeat`, `CardMetrics`, `LineChars`,
  `CARD_CSS`). `notifications.banner_entry_direction = "from_below"` is no longer drawn:
  quire's stack enters from the right only.
- **F361 The center.** Quire's `Panel { label, shown, on_hidden, onclose, width:
  Px(center_width_px), edge: Right, material: Popover, scrim: PanelScrim::None }` inside a
  wrapper carrying `NotificationMetrics` (filling the root, `pointer-events:none`, so the
  catcher under it still hears a press outside). `shown` follows the phase machine (`Up` is
  `Visible`, `Leaving` `Hidden`); the panel plays `PanelIn`/`PanelOut` and its `on_hidden` feeds
  `ExitSettled`, which unmaps: sill's `use_motion_timer(PaneOutR)` is gone. Each group is quire's
  `GroupHeader { icon, name, count, expanded, on_toggle, on_clear }` (the app's icon at 16 and
  name from the card's `use_app_mark`, F322) over its cards (`Swipe::Off`). The key rows are now
  each group's header then its cards (`CenterRow::Head` replaces the fold line: Return on a
  header opens or folds the group, Delete clears it). The panel takes no id, so the blur element
  is the column inside it (`PANEL_ID`) grown by the panel's padding (`PANEL_PADDING`, 12).
  Deleted: the `Surface{Popover}` one-cell-grid panel, the `PaneInR`/`PaneOutR` slide (F323),
  the `SectionHeader` stand-in and the fold line (F324), `fold_label`, `group_title`.
- **F362 Rich bodies.** `notification_card::rich_body` maps the service's parsed runs onto
  quire's: a link `RichRun::Link`, else one tone per run, the strongest of bold (`Strong`),
  italic (`Italic`), underline (`Underline`); an empty body is `None`. A link's press stays at
  the link; `on_link` logs it (`banners.link=<href>`), since sill has no URL opener (F321).
- **F363 Tests.** `banners_render` (8: slides in and times out, hover pauses and resumes, a
  press invokes `default`, the close button dismisses, a 100 px drag sends `SwipeAway` and a
  slow 40 px one springs back with its click swallowed, a link keeps its press, grouping, a
  dropped card and another output), `notification_card_render` (3), `notification_center_render`
  (11), `notification_center_keys` (5), and both lint suites, all on quire's classes; the pure
  tables for the machine, the deck, the regions, the rows, the body, the metrics and the count.
  In the stack a leaving card keeps its place until it has gone, so five apps after one show
  the old card first while it slides out.
- **F364 G100 and G101 closed** (shell-host 902a12c: an enter moves the document's pointer; an
  unmap, hide or close withdraws it). `dev/accept-notifications-real-input.sh` drops the 1 px
  nudge (`point` is one motion) and runs its steps in their natural order (the click on a body
  before the close button). `repro`: "G100: one motion onto the card: hovered yes; after a second
  motion: yes" PASS; "G101: a card after the column unmapped under the pointer, one motion onto
  it: hovered yes" PASS.

### Acceptance

`dev/accept-notifications.sh` (nested cosmic-comp 2560 x 1600, private bus): 7/7.

```
PASS step_warm                sill-banners-WINIT-0 spawned at start, hidden
PASS step_shows_holds_hides   shown 189 ms after the notification; hidden at 5648 ms (limit
                              5850); banners 0, kept 1
PASS step_hover               hovered at 4000 ms: still up at 7000 ms; gone 1796 ms after the
                              pointer left (limit 2150)
PASS step_grouping            three from one app: 140 px tall, banners 3; five apps: 250 px
PASS step_dnd                 a normal one kept without a banner (2 -> 3); a critical one up
                              after 10 s
PASS step_click               a click on the body: the card left in 376 ms, kept 4 -> 3
PASS step_idle                5 s idle: 0 frames
```

`dev/accept-notification-center.sh` (same compositor): 6/6. "N more" and Clear are now found in
the first group header's ink (`header_buttons`), and the debug pointer leaves the surface after
the toggle so the hovered Quiet button's ground does not run into Clear.

```
PASS step_warm, step_toggle_open_close (box 2168 8 2552 1492 in a 2560 1500 surface), step_clock
PASS step_groups   kept 3; '1 more' at 2472 71 opened Mail's group (6 ink rows collapsed, 7
                   opened); Mail's Clear at 2515 71: kept 1
PASS step_outside, step_idle (0 frames open, 0 hidden)
```

`dev/accept-notifications-real-input.sh` (headless KWin 1920 x 1080, real pointer and keys):
11/11.

```
PASS step_holder, step_hover (gone 1801 ms after the pointer left, limit 2150)
PASS step_click_default   a real click on the body: left in 110 ms, kept 2 -> 1; ActionInvoked
                          (uint32 2, 'default')
PASS step_close_button    hovered, the close button shows; a real click on it at 1555,43: gone
                          in 121 ms, kept 2 -> 1
PASS step_swipe           a real 90 px drag right: gone in 107 ms; banners 0, kept 2
PASS step_spring_back     a real 50 px drag, slowed: back at x 1552, banners 1
PASS step_center_clock_escape, step_center_outside, step_center_delete (kept 4 -> 2),
     step_idle (0 frames), step_holder
```

`step_spring_back` now reads the card's box with the pointer off it: hovered, quire's close
button widens the box by its 6 px overhang. Screenshots retaken: `dev/out/banners-cosmic.png`
(quire's card), `dev/out/banners-stack-cosmic.png` (three cards, Ada's group with its count chip
and layers), `dev/out/notification-center-cosmic.png` (quire's panel: MAIL 2 with "1 more" and
Clear over its stacked card, CHAT 1), `notification-center-open-cosmic.png`,
`notification-center-empty-cosmic.png`.

### Gaps found adopting the notification parts

- **Q170 (quire; Blitz `refresh_hover`) A card that slides in under a resting pointer is never
  hovered by moving on it.** After each resolve Blitz re-resolves the hovered node against the
  last pointer position (`Document::refresh_hover`) without dispatching pointerenter or
  pointerleave (its own TODO). A banner that slides in under the pointer (the next card after a
  clicked one, a newer one in the same place) becomes the hovered node silently; later motions
  over the same node change nothing, so quire's `NotificationCard` never hears `Hover::Over`:
  no hold pause, no close button, until the pointer leaves the card and comes back. Reproduced
  in the harness (a hovered card removed, the next one mounted under the pointer, a move of
  20 px: `data-hover` stays `off`) and live: `repro_card_under_resting_pointer` in
  `dev/accept-notifications-real-input.sh repro` FAILs with "a card slid in under the resting
  pointer, moved on: hovered no; out and back: yes". Wanted from quire: the card treats a
  pointermove with the pointer over it as `Over` when it thinks `Away` (or the host synthesizes
  enter and leave for hover changes that come from layout).

## Q170 closed (2026-09-26)

- shell-host 0e02bdf (F59) repairs the hover Blitz's `refresh_hover` moves silently at the end
  of a resolve. `dev/accept-notifications-real-input.sh repro` on the headless KWin: G100 "one
  motion onto the card: hovered yes", G101 "one motion onto it: hovered yes", Q170 "a card slid
  in under the resting pointer, moved on: hovered yes". All three stay as regression checks.

## M7: the lock screen and the polkit prompt, borrowed (2026-09-26)

design/20 §1.9 and §1.10: an existing locker and an existing polkit agent in a sill session
(sill's own are M11), and the session script that makes a real login possible. Numbered from
F380/G110. COSMIC's sources were read at `epoch-1.8.0` (cosmic-greeter, cosmic-idle, cosmic-osd,
cosmic-session, cosmic-comp's `session_lock.rs`; all GPL-3.0) for facts only, note-and-close;
nothing was pasted and nothing was put in a prompt.

### What COSMIC's parts do (verified)

- **F380 cosmic-greeter is COSMIC's locker.** One binary: run as the `cosmic-greeter` user it is
  the greetd greeter, as anyone else it is the lock screen (`--help` only knows `-h`/`-v`).
  cosmic-session starts it as a resident component. With logind reachable on the system bus it
  takes a sleep delay inhibitor ("COSMIC Greeter needs to display a lock screen"), finds its
  session by its **parent's** pid, and locks on that session's `Lock` signal and on
  `PrepareForSleep(true)`, unlocks on `Unlock`, and never exits. With the system bus unreachable
  it locks at once and exits 0 after a successful unlock. It speaks only `ext-session-lock-v1`
  to the compositor, so it locks any compositor that has it. Its logs go to the journal
  (tracing-journald), where it writes "session locked" once the lock is granted; nothing reaches
  stderr. It does not handle a refused lock (`finished`): it would sit with no surfaces.
  Verified on the nested 1.8.0: run with `DBUS_SYSTEM_BUS_ADDRESS` pointing nowhere it locked in
  about 0.4 s, a second lock was refused while it ran, and the journal had "session locked"
  (plus a harmless `pam_fprintd` "Error with getting the bus": the password prompt follows).
  Its lockfile (`$XDG_RUNTIME_DIR/cosmic-greeter-$XDG_SESSION_ID.lock`) is written only on the
  logind path, so it is no ready signal in the one-shot form.
- **F381 cosmic-comp keeps a session locked after its locker dies,** until another client locks
  and unlocks (its handler accepts a new lock once the old client is gone). So sill never kills
  its locker, not even when sill exits, and a slow one is taken as locked after 10 s rather than
  killed. `examples/session_locker.rs` (a password-free `ext-session-lock-v1` client for tests)
  is the recovery on the nested compositor: `session_locker 0` locks and unlocks at once.
- **F382 cosmic-idle** (config `com.system76.CosmicIdle` v1: `screen_off_time`,
  `suspend_on_ac_time`, `suspend_on_battery_time`, milliseconds, `None` = never; defaults 15
  min, 30 min on AC, 15 min on battery) fades the outputs to black on `ext-idle-notify-v1`,
  turns them off (`wlr-output-power`), and 0.5 s later runs the `LockScreen` system action
  (COSMIC's default `loginctl lock-session`); suspend runs the `Suspend` action (`systemctl
  suspend`). There is no lock time of its own: idle lock is screen-off time. It also serves
  `org.freedesktop.ScreenSaver` inhibits. It needs no cosmic-session.
- **F383 cosmic-osd is the polkit agent** (it registers `/com/system76/CosmicOsd` for its own
  logind session), but also the logout / restart / shutdown dialogs and the volume and
  brightness indicators, and it shows its volume indicator, with a sound, on every sink volume
  change, whoever made it.
- **F384 cosmic-session** starts cosmic-comp and hands it the other components' socket, imports
  and exports the systemd environment, starts `cosmic-session.target`, holds logind's
  `handle-power-key` inhibitor, owns `com.system76.CosmicSession` (`Exit`, `Restart`), starts
  orca when a11y is on, and restarts panel, launcher, app library, workspaces, osd, bg, greeter,
  files applet, idle and notifications forever. docs/cosmic-gaps.md "What cosmic-session does
  that `dist/sill-session` loses" goes row by row.

### What sill does

- **F385 The `locker` service** (`sill-services/src/locker/`, `Services.lock` /
  `locker_control`). `session.locker` (`auto`, `off`, `cosmic-greeter`, `swaylock`, `hyprlock`,
  or a command line for `/bin/sh -c`) is resolved by `choice::resolve` against the kind of
  session (`Host`) and what is on `PATH`:

  | Host | how it is told | `auto` resolves to | logind's Lock / PrepareForSleep |
  | --- | --- | --- | --- |
  | sill's session | no cosmic-session, `XDG_CURRENT_DESKTOP` names COSMIC | cosmic-greeter, else swaylock, else hyprlock, else off | followed |
  | cosmic-session | a `cosmic-session` process of this user | `Delegate`: sill's Lock calls logind `LockSession` for COSMIC's resident locker (the pre-M7 behaviour) | ignored (the resident locker has them) |
  | another desktop (nested under KDE) | anything else | the same programs as a sill session | ignored: sill never locks on another desktop's behalf |

  How each known locker is run and heard: cosmic-greeter with `DBUS_SYSTEM_BUS_ADDRESS` pointing
  nowhere (F380's one-shot form) and `RUST_LOG=warn,cosmic_greeter::locker=info`, ready when
  `journalctl --follow _PID=<pid>` shows "session locked"; swaylock with `--ready-fd 1`, ready
  on its first stdout line; hyprlock and commands, ready after 1 s. The cost of the one-shot
  cosmic-greeter: its lock screen's battery and network icons and its suspend / restart /
  shut-down buttons need the system bus and do nothing.
- **F386 The machine** (`locker/machine.rs`, table-tested): phases Unlocked / Locking / Locked,
  and a sleep hold Released / Armed / Delaying. Lock requests from sill's action always count;
  from logind only when the host is followed. Never a second locker while Locking or Locked, and
  a locker of the same process name already running (a previous sill's, or one started by
  hand) is adopted (`/proc` scan) and watched until it exits. The delay inhibitor
  (`Inhibit("sleep", "sill", "lock before sleep", "delay")`) is taken as soon as a locker is
  configured in a followed session and again after every resume, **not** on
  `PrepareForSleep(true)`: logind only waits for delay inhibitors that were held when the sleep
  began, so one taken on the signal is too late. On `PrepareForSleep(true)` the locker starts
  and the inhibitor is closed once it reports locked, it exits, or 4 s pass (under logind's
  default `InhibitDelayMaxSec` of 5 s). Unlock is the locker's; sill sees its exit.
- **F387 Deviation: sill's Lock no longer calls logind.** The launcher row and the power menu
  still send `SessionCommand::Run(Lock)`; the session service now forwards it to the locker
  service instead of calling `LockSession`, because sill nested under KDE shares KDE's logind
  session, and `LockSession` there locks the user's real desktop. Under cosmic-session the
  locker service still calls `LockSession` (`Delegate`). `Session.lock`'s offer is unchanged
  (it says whether logind knows this session, which no longer decides anything); the power
  menu does not show Lock. `session::spawn` takes the locker's `Control` (a signature change).
- **F388 Freeze amendment: `Request::Lock`** (`sill lock`, wire `{"kind":"lock"}`, answered
  `Done`), for scripts and for a `LockScreen` system action that bypasses logind. The shipped
  `system_actions` keeps COSMIC's `loginctl lock-session` (it works in both session kinds) and
  gains `LogOut: "sill power-menu open"` and `PowerOff: "sill power-menu toggle"` (Super+Shift+
  Escape and the power key), since cosmic-osd's dialogs are not there in a sill session.
- **F389 Idle: cosmic-idle, no sill idle keys.** The simplest route that keeps every timeout
  configurable: cosmic-idle already watches `ext-idle-notify-v1`, its three times are COSMIC's
  config (COSMIC Settings' Power page edits them), and its lock goes through `loginctl
  lock-session`, which sill's locker follows in a sill session. `session.idle_lock_minutes` /
  `session.idle_display_off_minutes` are not added: sill would own them only with its own idle
  timer, and the one thing that would buy (a lock time apart from screen off) is in
  docs/cosmic-gaps.md instead.
- **F390 polkit: `sill polkit-agent`** resolves `session.polkit_agent` (`auto`, `off`, a known
  agent's name, or a command) with `polkit::resolve` and execs it (`print` shows the choice).
  **Deviation from the brief's order:** `auto` tries polkit-kde (this machine:
  `/usr/libexec/kf6/polkit-kde-authentication-agent-1`), polkit-gnome, lxqt-policykit,
  polkit-mate, and cosmic-osd **last**, because cosmic-osd brings its own volume indicator
  (F383) next to sill's OSD. `session.polkit_agent = "cosmic-osd"` still picks it. sill
  implements no agent in M7. A polkit agent registers per logind session, so running one nested
  under KDE (whose agent already holds the session) is pointless; only the session script runs
  it.
- **F391 `dist/sill-session`**: two stages (outside: a login shell once, the environment, `exec
  cosmic-comp "$0" --inside`; inside: the kiosk child, which must never exit because cosmic-comp
  exits with it), `dbus-update-activation-environment --systemd`, `sill-session.target` (new,
  binds `graphical-session.target` and XDG autostart like `cosmic-session.target`), a
  `systemd-inhibit --what=handle-power-key` holder, and cosmic-settings-daemon,
  xdg-desktop-portal-cosmic, cosmic-idle, `sill polkit-agent` and `sill run`, each restarted on
  exit and given up after five exits within 10 s each. Kept out: cosmic-panel, cosmic-launcher,
  cosmic-app-library, cosmic-bg, cosmic-osd, cosmic-notifications, cosmic-workspaces,
  cosmic-files-applet, and a resident cosmic-greeter. `dist/sill.desktop` is the
  wayland-sessions entry (`DesktopNames=COSMIC`, as COSMIC's own). Documented, not installed;
  `sh -n` clean (shellcheck is not on this machine).
- **F392 Dependencies:** tokio gains its `process` and `io-util` features (the locker child and
  its stdout / journal lines); no new crate. `cargo deny check licenses` unchanged.
- **G110 (sill) sill nested under KDE follows nothing from logind,** by design (F385), so a
  real suspend of this machine does not lock the nested compositor; only `sill lock` and the
  Lock row do. In a sill login everything is followed.

### Settings (proposed design/22 rows; `AWAITING_ROWS` in `sill-settings/tests/keys.rs`)

A new `session` domain (`sill-settings/src/session.rs`, page Accounts, both Advanced):

| Key | Type | Default | Range / alternatives | Source | Status |
| --- | --- | --- | --- | --- | --- |
| `session.locker` | string | `"auto"` | `auto` (cosmic-greeter, else swaylock, else hyprlock; under cosmic-session COSMIC's own), `off`, `cosmic-greeter`, `swaylock`, `hyprlock`, or a command line run by `/bin/sh -c` | sill M7 (design/20 §1.9 borrowed locker) | proposed |
| `session.polkit_agent` | string | `"auto"` | `auto` (polkit-kde, polkit-gnome, lxqt-policykit, polkit-mate, then cosmic-osd), `off`, one of those names, or a command line | sill M7 (design/20 §1.10 borrowed agent) | proposed |

### Tests

Pure: `locker::machine` (31 transitions, deadlines, a whole sleep in order),
`locker::choice` (setting words, kinds of session, the resolution table, each locker's ready
signal), `polkit` (words, the resolution table), `sill::session::command_line`, the CLI and
wire lines for `lock`, the settings round trip, diff and defaults for `session`.

Live, on the nested cosmic-comp 1.8.0 (`wayland-1`; the real session was never locked and
nothing called logind's Lock or Suspend), `cargo test -p sill-services --test locker_live --
--ignored --test-threads=1 --nocapture`:

```
cosmic_greeter_locks_the_nested_compositor_and_its_exit_is_seen
  sill sees Lock { phase: Unlocked, pid: None }
  sill sees Lock { phase: Locking, pid: Some(Pid(1694675)) }
  sill sees Lock { phase: Locked, pid: Some(Pid(1694675)) }
cosmic-greeter reported locked after 407.684064ms       (its journal line)
  [a second ext-session-lock is refused while it runs: exit 3]
  sill sees Lock { phase: Unlocked, pid: None }         (killed: it cannot unlock without the
sill saw cosmic-greeter exit                             user's password)
recovery lock-and-unlock: exit Some(0)                  (F381)
the_stand_in_locks_and_unlocks_the_nested_compositor
  sill sees Lock { phase: Locking, pid: Some(Pid(1694879)) }
  sill sees Lock { phase: Locked, pid: Some(Pid(1694879)) }
locked after 12.096776ms                                (a second Lock changes nothing)
  sill sees Lock { phase: Unlocked, pid: None }
unlocked after 1.513542066s                             (the locker unlocked and exited)
test result: ok. 2 passed
```

End to end through the daemon (`sill run` on `wayland-1` with a scratch `XDG_CONFIG_HOME` whose
`session.locker` is the stand-in with a 2 s hold, then `sill lock`): `{"kind":"done"}`, a second
lock refused while it held (exit 3), free again after (exit 0), `sill quit` exit 0.

What needs a real login (lock before sleep, idle lock, `pkexec true`, the power key) is in
docs/manual-checks.md "Needs a sill login".

## M10 freeze (2026-09-26)

The calendar, the screenshot thumbnail, widgets in the notification center and on the desktop,
and hot corners (design/20 §1.12, §1.13, §1.14, §1.16), frozen on `m10-freeze` so four agents
can fill them in parallel and a fifth can prove them under real input. Everything compiles;
every pure function a later agent fills is `todo!()` with its table test `#[ignore = "M10 fill:
<lane>"]` (35 of them), every component draws nothing yet (the notification center's render
tests mount the widget slot, so no component body is a `todo!()`), and every daemon follower a
lane fills is a no-op stub, so the suite stays green and `sill run` starts. Findings are numbered
from F400/Q180/G120; the lanes take F420/Q190/G130 (a), F440/Q200/G140 (b), F460/Q210/G150 (c),
F480/Q220/G160 (d) and F500/Q230/G170 (e).

macOS is the reference throughout: since macOS 11 the clock opens Notification Center, whose
widgets include the month and Up Next; Sonoma puts the same widgets on the desktop; a screenshot
floats a thumbnail at the bottom right; Hot Corners live on the "Desktop & Dock" page.

- **F400 Dependencies: none added.** `jiff` (pinned block, already sill-services') joins
  sill-surfaces for the month grid and the world clock; `notify`, `image` and `dirs` are already
  sill-services'. Not added: `icalendar` (brings chrono, expands no `RRULE`), `rrule` (chrono and
  chrono-tz: a second compiled-in zone database beside jiff's), a D-Bus crate for
  evolution-data-server (zbus is here when it is wanted). `cargo deny check licenses` unchanged.
- **F401 Where calendar events come from (researched).** On this machine: KDE's Akonadi runs,
  and KOrganizer's default calendar is its iCal resource, a plain file named in
  `~/.config/akonadi_ical_resource_0rc` (`Path[$e]=$HOME/.local/share/apps/korganizer/std.ics`;
  empty here); evolution-data-server and GNOME Online Accounts are not installed (GOA's
  calendars reach clients only through EDS anyway). In general: vdirsyncer syncs any CalDAV
  server (Nextcloud, Fastmail, iCloud, Google) into a directory of one `.ics` per event per
  collection (khal's `~/.calendars/<collection>/`), and every calendar exports `.ics`. So the
  cheap, local route that covers the most is **`.ics` files and vdir directories**, parsed by
  hand (RFC 5545's unfolding, `VEVENT`, `DTSTART`/`DTEND`/`DURATION`, dates, UTC, `TZID` by name
  through jiff's IANA database, the common `RRULE` subset, `EXDATE`, overrides); EDS
  (`org.gnome.evolution.dataserver.Sources5`/`Calendar8`, whose `GetObjectList` answers
  iCalendar text the same parser reads) is a later provider behind the same trait.
- **F402 The events service** (`sill_services::events`). `EventsProvider` (a seam with two
  implementations today, `IcsProvider { places }` and `NullProvider`, and EDS later; blocking,
  run on the pool) reads a `Window` and names the files it `watched()`. `calendar.sources`
  (`auto`, `off`, or `;`-separated files and directories) parses to `EventSources`; `auto` is
  KOrganizer's file (`akonadi_ical_path`), `$XDG_DATA_HOME/sill/calendars` and `~/.calendars`,
  whichever exist (`auto_places`, `provider_for`). The service publishes an `Agenda { today,
  events, status }` (`reduce_agenda`), `today` republished at local midnight so the month moves
  on, events held for `held_window(today)` (five weeks back, ten ahead); `up_next` picks the
  next events. **Passive until the daemon's first `EventsCommand::Sources`**
  (`crates/sill/src/calendar.rs`), so a test or a render never reads a calendar.
- **F403 The calendar is a widget, not a popup** (deviation from design/20 §1.12's xdg popup
  from the clock, which M6 already replaced by the notification center, F266). `Calendar` is
  the month (quire's `MonthGrid` once it exists, Q180; `widgets::calendar::grid::month_grid`
  computes it: rows of ISO-numbered weeks from `calendar.first_weekday`, the neighbours' days
  padding, today marked, busy days marked, four to six rows; `shift_month` for previous and
  next), and `UpNext` is macOS's "Up Next" list (`event_lines`: "All day", "Now", "10:30 –
  11:00", "Tomorrow 09:00", "Mon 09:00").
- **F404 Widgets are one enum, drawn in two hosts** (`sill_surfaces::widgets`). `WidgetKind`
  (settings: `Calendar`, `UpNext`, `Battery`, `NowPlaying`, `WorldClock`) is a closed set chosen
  in the settings, so an enum, not a trait (CONVENTIONS §0); `Widget { kind, host }` picks the
  kind's component and wraps it in `WidgetFrame`; `WidgetHost::{Center, Desktop}`,
  `WidgetSize::{Small, Medium, Large}` (1x1, 2x1, 2x2 desktop cells; everything spans the
  center's column), `size_of`, `widget_element_id`, and `widgets_css()` (one stylesheet file per
  widget, so lanes a and b never share a file). Weather is not a kind: nothing provides weather
  (adding it later is additive). Battery is this machine's UPower battery and every connected
  Bluetooth device with a `Battery1` level (`battery_rows`); Now Playing reuses M5's module;
  World Clock reads `widgets.world_clocks` (`parse_zones`, `clock_rows`).
- **F405 The desktop widget layer is a skeleton** (`surfaces::desktop_widgets`, lane b). One
  surface per output while `widgets.desktop` is on, on layer **`Bottom`**, not a second
  `Background` surface above sill's wallpaper as the brief put it: layer-shell does not order two
  surfaces on one layer, so the widgets could be drawn under the wallpaper (and under cosmic-bg's
  with `spaces.wallpaper_drawer = "cosmic"`); `Bottom` is above every background and below
  windows, which is design/20 §1.14's row. All four edges, exclusive `Auto`, keyboard `None`,
  input and blur empty until the surface sets them to its frames (`RegionSpec::Elements`), so a
  click between widgets reaches the desktop. One surface per output rather than design/20's "one
  surface per widget": placement is a grid inside it (`grid_size`, `place`: saved places kept
  when they fit, new ones from the top right), saved in `$XDG_STATE_HOME/sill/
  desktop-widgets.json` (state, not a setting, like the dock's pins).
- **F406 How sill learns a screenshot happened (researched).** COSMIC's `cosmic-screenshot`
  1.8.0 calls the portal (`org.freedesktop.portal.Screenshot`, xdg-desktop-portal-cosmic), whose
  picker saves `Screenshot_%Y-%m-%d_%H-%M-%S.png` into Pictures (its `save_location`: Pictures,
  Documents or the clipboard); with `--interactive=false` cosmic-screenshot saves the whole
  screen into `--save-dir` itself. Nothing is announced on the bus: the portal's `Response` goes
  to its caller only, and `--notify` is an ordinary notification. So the simplest route that sees
  every screenshot, whoever takes it (COSMIC's Print key, `sill screenshot`, spectacle, grim), is
  **an inotify watch of the screenshot folders** (`screenshot.folders`, `auto` = Pictures and
  Pictures/Screenshots): a file closed after writing, or renamed into place, whose name reads as
  a screenshot (`is_screenshot_name`: COSMIC's, GNOME's and spectacle's names, never a hidden or
  partial file). `sill screenshot [area|window|screen]` only runs `screenshot.tool`
  (`tool_command`; `auto` = cosmic-screenshot, whose picker cannot be told area or window, so
  both open it, and `screen` is `--interactive=false --save-dir`); its file comes back through
  the same watch, so there is one path to test. A clipboard-only screenshot has no file and no
  thumbnail, as on macOS.
- **F407 The screenshots service** (`sill_services::screenshots`): `Shots { current, watching,
  taking, last_id }`, `reduce_shots` returning `ShotEffect::{MakePreview, Open, Trash, RunTool}`
  (xdg-open, `gio trash`, a 480 px PNG preview in `$XDG_CACHE_HOME/sill/shots/preview_name`),
  `ShotCommand::{Configure, Take, Announce, Leave(id, ShotLeave::{Opened, Kept, Trashed,
  Dragged})}`. **Passive until the daemon's first `Configure`** (`crates/sill/src/
  screenshots.rs` `spawn_policy`), so a test never watches `~/Pictures` or runs a tool; tests
  configure temp folders and a stand-in tool command.
- **F408 The thumbnail** (`surfaces::screenshot_thumb`, design/20 §1.13): one warm hidden
  Overlay per output (`thumb_layer`: `BOTTOM | RIGHT`, exclusive `Auto` so it sits above the
  dock's reserve, keyboard `None`, input and blur `Element("thumb")`), shown on the banners'
  output (`BannerLink`, the main display's stand-in), `THUMB_WIDTH` 240 until quire's card
  (Q181). `machine::step`: hold `screenshot.thumbnail_hold_ms` (5200), paused under the pointer
  and 1500 ms more after it leaves; a click opens, a swipe keeps, Delete trashes, a drag out
  hands the file over, a newer shot replaces. **Drag out is frozen as a hook**:
  `start_file_drag` answers `Unsupported` until shell-host has a drag source (G120).
- **F409 Hot corners** (`surfaces::hot_corners`, design/20 §1.16): a surface per active corner
  (`hot_corners.<corner>` not `None`) per output. `corner_layer`: Overlay, anchored to the
  corner, `hot_corners.size_px` (2) square, exclusive **`Ignore`** so it is at the true corner
  over the bar's and the dock's reserve, keyboard `None`, input `Whole`, no blur, never painted.
  `machine::step` (`Armed`, `Dwelling`, `Blocked`, `Spent { pointer, rearm }`): the dwell
  (`dwell_ms`, 150) fires once, a required modifier (`modifier`) must be held when it ends, and
  it re-arms only after the pointer has left and `rearm_ms` (500) has passed. A firing goes to
  the `CornerLink` (in `crate::links`, every root's context); the daemon carries it out
  (`corners::corner_effect`): Launcher, Notification Center and Control Center through their
  links, Lock through the locker, Show Desktop through the toplevel manager (`show_desktop`:
  minimise the active workspace's windows, and the next firing brings them back), Workspaces as
  `cosmic-workspaces` (COSMIC's own `WorkspaceOverview` action runs exactly that), Command as
  `/bin/sh -c`. Defaults are all `None`, as macOS ships them (but for Quick Note), so a fresh
  install spawns no corner surface.
- **F410 IPC and CLI.** `Request::Screenshot(ScreenshotRequest::Take(ShotArea))`,
  `Request::Widgets(WidgetsRequest::ToggleDesktop)`, `DebugRequest::Screenshot(FakeScreenshot {
  path })` (as if the watch saw the file: nothing is captured) and
  `DebugRequest::Corner(FakeCorner { corner })` (a hit on the first output, as if the dwell ran
  out); wire spellings pinned in `sill-ipc/tests/serde.rs`. CLI (`cli_desktop.rs`): `sill
  screenshot [area|window|screen]` (default `area`), `sill widgets toggle-desktop`, `sill debug
  screenshot PATH`, `sill debug corner {top-left,top-right,bottom-left,bottom-right}`. Seen on
  the nested cosmic-comp (own runtime dir, private bus, scratch config): `sill debug stats` lists
  `sill-screenshot-thumb-WINIT-0` beside the rest (no corner or desktop surface: none is on);
  `screenshot` and `debug screenshot` answer "screenshots: the service has stopped" until lane c
  fills the loop; `debug corner top-left` answers done (the hit is dropped until lane d);
  `widgets toggle-desktop` answers "not built yet (M10 lane b)"; `quit` exits 0.
- **F411 Daemon wiring.** `OutputSurfaces` gains `thumbnail`, `desktop_widgets` and `corners:
  CornerSurfaces` (per-corner `get`/`set`); `Links` gains `corners: CornerLink`; `run_event`
  calls `screenshots::spawn_missing` (implemented: warm, hidden), `widgets::spawn_missing` and
  `corners::spawn_missing` (lanes b and d) before the OSD's; `run` starts
  `calendar::spawn_policy`, `screenshots::spawn_policy` and `corners::spawn_hits` (stubs until
  lanes a, c and d); `collect_stats` names the new surfaces. `Services` gains `agenda`,
  `events_control`, `shots`, `shots_control`; hooks `use_agenda`, `use_shots`; the surfaces'
  `tests/support/fake.rs` feeds both.
- **F412 Settings.** Four new domains, all Advanced, in `AWAITING_ROWS` (row text in Q185):
  `widgets` (Page Dock: `center`, `desktop`, `desktop_widgets`, `desktop_cell_px`,
  `desktop_gap_px`, `world_clocks`), `calendar` (Page Notifications: `sources`,
  `first_weekday`, `week_numbers`, `up_next_days`, `up_next_max`), `screenshot` (Page Keyboard
  and Shortcuts: `thumbnail`, `thumbnail_hold_ms`, `folders`, `tool`) and `hot_corners` (Page
  Dock: the four corners, their four commands, `dwell_ms`, `rearm_ms`, `modifier`, `size_px`).
  `SettingsChange` gains `Widgets`, `Calendar`, `Screenshot`, `HotCorners`.
- **F413 Left out on purpose.** Weather (no source); moving desktop widgets by drag (plain
  pointer events in the layer, later); the thumbnail's Mark Up and OCR "Copy text" (design/20
  §1.13's actions; no markup tool or OCR here); the screenshot sound (`sound.event_screenshot`
  exists, the sound service is still inert); opening a calendar app from an event (none to
  name); EDS.

### The M10 fill-in: four agents, disjoint files, then real input

Every agent reads CONVENTIONS §0 and §1 (fill bodies, add private helpers and tests; never
change a frozen signature, public field or variant; stop and report instead), ORCHESTRATION
"Coherence rules", design/20 §1.12-§1.14 and §1.16, quire CONSUMING.md, and this section.
Everyone runs, before reporting:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features sill/debug
./scripts/check-boundary.sh
cargo deny check licenses
```

(`cargo fmt -p <crate>` while the wave runs.) Safety, for every test and script: **never capture
the user's real session, never write to `~/Pictures`** (temp folders only; the screenshot
service is configured with `ShotFolders::At(temp)` and a stand-in tool), **never touch
`wayland-0`**. Acceptance runs on the nested cosmic-comp (`wayland-1`: `WAYLAND_DISPLAY=wayland-1
wayland-info` answers first; it is never stopped or restarted), the daemon with its own
`XDG_RUNTIME_DIR` (the nested socket as an absolute `WAYLAND_DISPLAY`), a scratch
`XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME`, under `dbus-run-session
--config-file=dev/private-bus.conf`, as `dev/accept-notification-center.sh` does. Files not
listed under an agent are read-only to all: in particular `sill-services/src/{lib.rs,
services.rs}`, `sill-surfaces/src/{lib.rs,hooks/**,widgets/mod.rs}`,
`sill-surfaces/tests/support/fake.rs`, `sill-settings/**`, `sill-ipc/**`, and
`crates/sill/src/{daemon.rs,cli.rs,cli_desktop.rs,links.rs,outputs.rs,lib.rs}`.

**(a) a-calendar: the events provider, Calendar and Up Next.**
- Owns: `crates/sill-services/src/events/**` (the implemented `NullProvider`,
  `IcsProvider::watched`, `provider_for`, `held_window`, `Agenda::starting` and `week_start` may
  change inside, never in signature), new `crates/sill-services/tests/events_live.rs` and
  `tests/fixtures/calendars/**`; `crates/sill-surfaces/src/widgets/calendar/**`,
  `widgets/up_next.rs`, `style/widget_calendar.css`, `style/widget_up_next.css`, a new
  `crates/sill-surfaces/tests/calendar_render.rs`; `crates/sill/src/calendar.rs`; a new
  `dev/accept-calendar.sh`.
- Frozen: every `pub` item `events` re-exports, with its fields and variants (`Agenda`,
  `AgendaEvent`, `CalEvent`, `EventTime`, `EventUid`, `CalendarName`, `SourceStatus`, `Window`,
  `EventsCommand`, `EventsProvider`, `ProviderRead`, `EventsError`, `EventSources`, `IcsPlace`,
  `AutoEnv`, `Days`), `ics::{IcsEvent, IcsTime, IcsEnd, IcsStatus, IcsError}`,
  `rule::{Rule, Freq, RuleEnd}`, the signatures of `reduce_agenda`, `parse_sources`,
  `akonadi_ical_path`, `auto_places`, `unfold`, `parse_ics`, `parse_rule`, `occurrences`,
  `up_next`, `spawn`; `grid::{Month, Step, IsoWeek, DayPlace, DayMark, Eventful, GridDay,
  GridWeek, MonthGrid, shift_month, month_grid, busy_days}`, `CalendarWidget { host }`,
  `UpNextWidget { host }`, `EventLines`, `event_lines`, both `CSS`; `calendar::{sources_of,
  spawn_policy}`.
- Done: every `M10 fill: a-calendar` test passes un-ignored; `live::run` waits for `Sources`,
  builds `provider_for` (reading `$HOME`, `$XDG_DATA_HOME` and Akonadi's rc), reads on the
  blocking pool in the system zone, watches `watched()` with `notify` (debounced), re-reads on a
  change, a `Refresh` and the day's turn, publishes through `reduce_agenda`, and ends when every
  receiver is gone; `spawn_policy` sends `sources_of` at start and on every change; the Calendar
  widget draws `month_grid` for the month shown (today's on mount), `IconButton{Tool}` previous
  and next, week numbers per `calendar.week_numbers`, busy days marked, with quire components
  and tokens only (the SSR class test and `ds::lint::stylesheet`); Up Next lists `up_next`
  through `event_lines`, an empty state, and a quiet line for `SourceStatus::{Off,
  NoCalendars}` and for problems; both redraw on the clock's minute tick. Live test
  (`#[ignore]`): a temp directory with a file and a vdir collection; an edit is re-read within
  1 s.
- Verify: the gates; `cargo test -p sill-services --test events_live -- --ignored`;
  `dev/accept-calendar.sh` on the nested compositor, the scratch config's `calendar.sources` a
  temp `.ics` (fixture events today, tomorrow and weekly) and `widgets.center = ["calendar",
  "up_next"]`: (1) after `sill notifications open`, a capture of
  `sill-notification-center-<out>` shows the month with today marked
  (`dev/out/calendar-cosmic.png`); (2) Up Next lists the fixture's next events in order; (3) an
  edit of the file shows within 2 s; (4) `calendar.week_numbers = "show"` and `sill reload`:
  the week column appears; (5) 0 frames idle while open. The slot is lane b's: until b is
  merged, (1)-(4) are render tests on `Widget { kind, host: Center }` and the script exits 2
  naming the missing slot.

**(b) b-widgets: the widget frame, the notification center's slot, the desktop layer.**
- Owns: `crates/sill-surfaces/src/widgets/{frame.rs,battery.rs,now_playing.rs,
  world_clock.rs}`, `style/widget_{frame,battery,now_playing,world_clock}.css`,
  `surfaces/notification_center/{widgets.rs,panel.rs}` (the panel only to inject
  `widgets_css()`), `surfaces/control_center/modules/now_playing.rs` (only to share its body,
  F404), `surfaces/desktop_widgets/**` (the implemented `layer.rs` inside only),
  `style/desktop_widgets.css`; `crates/sill/src/widgets.rs`; `tests/widgets_lint.rs`, new
  `tests/{widgets_render,desktop_widgets_render}.rs`; a new `dev/accept-widgets.sh`.
- Frozen: `WidgetFrame { kind, host, size, children }`, `BatteryOwner`, `BatteryRow`,
  `battery_rows`, `ZoneName`, `ClockRow`, `parse_zones`, `clock_rows`, `BatteryWidget`,
  `NowPlayingWidget` and `WorldClockWidget` (each `{ host }`), their `CSS`; `CenterWidgets` (no
  props); `DesktopWidgets { output }`, `desktop_layer`, `DESKTOP_WIDGETS_CSS`, `GridSlot`,
  `GridSize`, `Placement`, `grid_size`, `place`; `widgets::{spawn_missing, flipped, dispatch,
  namespace}` in `crates/sill`.
- Done: the ignored tests pass; `CenterWidgets` draws `Widget { host: Center }` for each of
  `widgets.center` in order, under the history, the column scrolling as a whole; the frame is
  quire's `Surface{Widget}` on the desktop and a tile on the Popover in the center (Q182: the
  corner and padding are the frame's, never sill CSS); Battery, Now Playing (M5's module, not
  redrawn) and World Clock drawn from their pure parts; `widgets::spawn_missing` spawns the
  desktop layer per output while `widgets.desktop` is `On` and closes it when `Off` (the daemon
  calls it after every event, a settings change included); the layer places `desktop_widgets`
  with `place` over `grid_size`, loads and saves the placements, and sets input and blur to the
  frames with `RegionSpec::Elements`; `sill widgets toggle-desktop` loads, flips (`flipped`),
  saves and publishes; quire components and tokens only.
- Verify: the gates; `dev/accept-widgets.sh` on the nested compositor: (1) with the default
  `widgets.center` (Calendar, Up Next, Battery), `sill notifications open` and a capture shows
  three frames under the history (`dev/out/widgets-center-cosmic.png`; Calendar and Up Next are
  empty until lane a is merged); (2) `sill widgets toggle-desktop` adds
  `sill-desktop-widgets-<out>` to `sill debug stats` with the Calendar and Battery frames
  (`dev/out/widgets-desktop-cosmic.png`), and a second toggle removes it; (3) the file's
  `widgets.desktop` flips each time; (4) 0 frames idle on the desktop layer.

**(c) c-screenshot: the screenshots service and the thumbnail.**
- Owns: `crates/sill-services/src/screenshots/**` (the implemented `save_folder` and
  `ShotKind::word` inside only), a new `crates/sill-services/tests/screenshots_live.rs` and its
  fixtures; `crates/sill-surfaces/src/surfaces/screenshot_thumb/**` (the implemented `layer.rs`
  and `drag.rs` inside only), `style/screenshot_thumb.css`, `tests/screenshot_thumb_lint.rs`, a
  new `tests/screenshot_thumb_render.rs`; `crates/sill/src/screenshots.rs`;
  `dist/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions` (a `Screenshot: "sill
  screenshot area"` line, once the tool path works); a new `dev/accept-screenshot.sh`.
- Frozen: every `pub` item `screenshots` re-exports, with its fields and variants (`Shots`,
  `Shot`, `ShotId`, `ShotPreview`, `ShotLeave`, `Watching`, `Taking`, `ShotEvent`, `ShotEffect`,
  `ShotCommand`, `ShotPolicy`, `ShotFolders`, `ShotTool`, `ShotKind`, `CommandLine`,
  `ShotName`), the signatures of `reduce_shots`, `is_screenshot_name`, `parse_folders`,
  `folders_of`, `preview_name`, `parse_tool`, `tool_command`, `spawn`; `ScreenshotThumb {
  output }`, `thumb_layer`, `thumb_region`, `THUMB_ID`, `THUMB_RADIUS`, `THUMB_WIDTH`, `ROOM`,
  `SCREENSHOT_THUMB_CSS`, `machine::{HoldToken, Pointer, ThumbState, ThumbParams, ThumbIn,
  ThumbOut, step}`, `drag::{DragStart, start_file_drag}`; `screenshots::{spawn_missing,
  kind_of, dispatch, fake, policy_of, spawn_policy, namespace}` in `crates/sill`.
- Done: the ignored tests pass; `live::run` per F406 and F407 (the folders that exist watched;
  a close after writing or a rename into place of a screenshot name; previews on the blocking
  pool; `xdg-open`, `gio trash`, the tool as a child, `TakeEnded` at its exit); `spawn_policy`
  sends `policy_of` at start and on change; the thumbnail follows `use_shots` and the
  `BannerLink` through `step`, woken by quire's timers only, entrance and exit in quire motion
  only (no keyframes of sill's own; Q181 until a thumbnail card exists);
  `screenshot.thumbnail = "hide"` keeps it hidden; input and blur on the card; the swipe is
  quire's `Swipe` if it can take a plain card (else the gap is Q210); a drag out calls
  `start_file_drag` and stays put on `Unsupported`.
- Verify: the gates; `cargo test -p sill-services --test screenshots_live -- --ignored` (a temp
  folder; a file written under a temp name and renamed to `Screenshot_…png` is `current` within
  500 ms with a preview; a `.part` file is nothing; a stand-in tool that copies a fixture in:
  `Take`, then the shot); `dev/accept-screenshot.sh` on the nested compositor, the scratch
  config's `screenshot.folders` a temp folder and `screenshot.tool` a stand-in that copies a
  fixture PNG there (never cosmic-screenshot, never the real Pictures): (1)
  `sill-screenshot-thumb-<out>` exists, hidden; (2) `sill debug screenshot
  <temp>/Screenshot_a.png` shows it within 500 ms (`dev/out/screenshot-thumb-cosmic.png`); (3)
  it hides after the hold plus the exit, and the file is still there; (4) `sill screenshot
  screen` (the stand-in) shows the new file; (5) `sill debug click` on the card runs the opener
  (a stand-in `xdg-open` first on `PATH`, logging its argument); (6) hovered at 4 s (`sill debug
  pointer`) it is up at 7 s and gone 1.5 s plus the exit after `sill debug leave`; (7) 0 frames
  while hidden.

**(d) d-corners: hot corners.**
- Owns: `crates/sill-surfaces/src/surfaces/hot_corners/**` (the implemented `layer.rs`,
  `link.rs`, `corner_action`, `corner_command`, `active_corners` and `held` inside only), a new
  `style/hot_corners.css` if the transparent element needs a rule, a new
  `tests/hot_corners_render.rs`; `crates/sill/src/corners.rs`; a new
  `dev/accept-hot-corners.sh`.
- Frozen: `Corner` (`ALL`, `word`), `HotCorner { output, corner }`, `corner_layer`,
  `CornerLink` (`new`, `hit`, `last`, `subscribe`), `CornerHit`, `machine::{Token, KeyState,
  Mods, Inside, Rearm, Phase, CornerMachine, CornerParams, CornerIn, CornerOut, step, held}`;
  `corners::{CornerEffect, corner_effect, Stash, show_desktop, corner_of, spawn_missing,
  spawn_hits, fake, namespaces}` and `outputs::CornerSurfaces` in `crates/sill`.
- Done: the ignored tests pass; `spawn_missing` spawns `corner_layer` for each of
  `active_corners` per output, closes a corner whose action became `None` and resizes on a
  `size_px` change; the surface draws one transparent element over the whole square (no `Ds`,
  no material) and feeds `step` from its pointer enter and leave, with quire timers for the
  dwell and the re-arm, calling `CornerLink::hit` on `Fire`; `spawn_hits` carries out
  `corner_effect` per F409 (the launcher on the hit's output, the center and the control center
  through their links, `crate::session::lock`, `show_desktop` over the output's active
  workspaces as `WlCommand`s, a child process for `Run`), and the report says what shell-host
  gives for modifiers (G121).
- Verify: the gates; `dev/accept-hot-corners.sh` on the nested compositor, the scratch config
  `top_left = "launcher"`, `top_right = "notification_center"`, `bottom_right = "command"` (a
  command that touches a temp file), `bottom_left = "none"`: (1) `sill debug stats` lists three
  corner surfaces and not the fourth; (2) `sill debug corner top-left` opens the launcher (its
  panel captures), `top-right` the center, `bottom-right` touches the file; (3) `sill debug
  pointer sill-hot-corner-top-left-<out> 0 0` opens the launcher after the dwell and not before
  (checks at 50 ms and 400 ms), a second pointer without a leave does nothing, a leave and 600 ms
  re-arm it; (4) `top_left = "none"` and `sill reload` remove its surface; (5) 0 frames on every
  corner surface. The script never sets a corner to `lock`.

**(e) e-real-input: after a, b, c and d are merged.**
- Owns: a new `dev/accept-m10-real-input.sh` and its entries in `docs/manual-checks.md`. No
  source file: a bug is reported against its lane's files.
- Done and verify: on shell-host's headless KWin (`scripts/dev/kwin-virtual.sh start`,
  `fake-input.sh`, one fake-input client for the whole run: G50), the daemon on KWin's private
  bus with the scratch config and temp folders above: (1) the real pointer moved into the
  top-left corner (`move 0 0`) opens the launcher after the dwell; `move 0 0` again without
  leaving does not; leaving and coming back after the re-arm does (this is the check that a 2 px
  transparent Overlay at the corner, over the bar, really gets pointer enter); (2) with
  `modifier = "super"`, the corner fires with `key Super_L down` held and not without, or the
  run records G121 as confirmed; (3) a stand-in screenshot shows the thumbnail; a real hover
  keeps it past the hold, a real click runs the stand-in opener, a real drag 90 px right
  dismisses it, a drag up-left leaves it in place (G120); (4) the notification center's widgets
  scroll with the history under a real wheel; (5) with the desktop layer on, a real click between
  widgets reaches the window beneath. KWin is stopped at the end.

### Gaps found in the M10 freeze

- **Q180 (quire)** `MonthGrid` (design/20 §1.12 and §3 open decision 5; into design/04 first):
  a month of day cells in 7 columns from a first weekday, an optional leading ISO week column,
  the neighbours' days quieter, today on the accent disc, a busy day's dot, weekday heads,
  optionally pressable days, and the month change on `slide-l`/`slide-r`. Its data is
  `widgets::calendar::grid::MonthGrid` (sill computes it, the component draws it). Until then
  lane a lays text out with tokens and says so.
- **Q181 (quire; closed, "Adopting the M10 parts")** A thumbnail card (design/20 §1.13): an image letterboxed in a Toast card,
  `rise --t-big --e-spring` in, `slide-r` out at `--e-exit`, a hover row of actions (Delete; Mark
  Up and Copy Text later), a drag ghost, and its width (sill's `THUMB_WIDTH` 240 meanwhile).
- **Q182 (quire; closed, "Adopting the M10 parts")** A widget frame (design/20 §1.14; `Material::Widget` exists): small, medium
  and large sizes on a grid unit, an optional title row with a glyph, the card's corner and
  padding, a variant as a tile inside a Popover (the notification center's widgets), and `bump`
  on a value change.
- **Q183 (quire; closed, "Adopting the M10 parts")** A world-clock face (an analog dial tinted for day and night, or macOS's
  digital row in the small size), and a battery level ring for the Battery widget if
  `ProgressRing` is not it.
- **Q184 (quire, design/13)** Hot corners have no behaviour section in design/13, though
  design/20 §1.16 cites "13 (dwell time and action table)": the dwell (sill proposes 150 ms),
  the re-arm (500 ms), the modifier rule and the action table belong there.
- **Q185 (design/22)** The rows `AWAITING_ROWS` exempts (F412): new sections `widgets`,
  `calendar`, `screenshot` and `hot_corners` after §3.19, which `tests/keys.rs` reads without
  change (everything up to "## 4. Rust shape" is parsed); §2's file table gains the four
  domains:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `widgets.center` | `Vec<WidgetKind>` (`Calendar, UpNext, Battery, NowPlaying, WorldClock`) | `[Calendar, UpNext, Battery]` | which widgets show under the notifications, in order | `20-SURFACES.md#1-14-desktop-widgets-spec-tier-2`; sill FINDINGS "M10 freeze" F404 | proposed (M10 freeze, 2026-09-26) |
  | `widgets.desktop` | `DesktopWidgets::{Off,On}` | `Off` | the desktop widget layer (macOS Sonoma); `sill widgets toggle-desktop` flips it | same; F405 | proposed (M10 freeze, 2026-09-26) |
  | `widgets.desktop_widgets` | `Vec<WidgetKind>` | `[Calendar, Battery]` | which widgets the desktop shows; where they sit is saved state (`desktop-widgets.json`), not a key | same; F405 | proposed (M10 freeze, 2026-09-26) |
  | `widgets.desktop_cell_px` | `Px` | `164` | `120..=240`; a small widget is one cell, a medium 2x1, a large 2x2 | same; F405 | proposed (M10 freeze, 2026-09-26) |
  | `widgets.desktop_gap_px` | `Px` | `16` | `0..=48`; between cells and around the grid | same; F405 | proposed (M10 freeze, 2026-09-26) |
  | `widgets.world_clocks` | `Text` | `""` | IANA zone names separated by `;` (a `Vec` of text is not derivable yet, as for `display.scale_overrides`) | same; F404 | proposed (M10 freeze, 2026-09-26) |
  | `calendar.sources` | `Text` | `"auto"` | `auto` (KOrganizer's file via Akonadi's iCal resource, `~/.local/share/sill/calendars`, `~/.calendars`), `off`, or `.ics` files and folders separated by `;` | `20-SURFACES.md#1-12-calendar-popup-spec-tier-2`; F401, F402 | proposed (M10 freeze, 2026-09-26) |
  | `calendar.first_weekday` | `FirstWeekday::{Monday,Sunday}` | `Monday` | the month's first column (ISO 8601) | same; F403 | proposed (M10 freeze, 2026-09-26) |
  | `calendar.week_numbers` | `WeekNumbers::{Hide,Show}` | `Hide` | a column of ISO week numbers | same; F403 | proposed (M10 freeze, 2026-09-26) |
  | `calendar.up_next_days` | `Count` | `7` | `1..=31`; how far Up Next looks ahead | same; F403 | proposed (M10 freeze, 2026-09-26) |
  | `calendar.up_next_max` | `Count` | `3` | `1..=10`; how many events Up Next lists | same; F403 | proposed (M10 freeze, 2026-09-26) |
  | `screenshot.thumbnail` | `ShotThumbnail::{Show,Hide}` | `Show` | the floating thumbnail after a screenshot (macOS's "Show Floating Thumbnail") | `20-SURFACES.md#1-13-screenshot-thumbnail-spec-integrated-experience-priority-pick`; F408 | proposed (M10 freeze, 2026-09-26) |
  | `screenshot.thumbnail_hold_ms` | `Ms` | `5200` | `1000..=30000`; paused while hovered (`ToastHold`) | same; F408 | proposed (M10 freeze, 2026-09-26) |
  | `screenshot.folders` | `Text` | `"auto"` | `auto` (Pictures and Pictures/Screenshots), `off`, or folders separated by `;`, watched for new screenshots | same; F406 | proposed (M10 freeze, 2026-09-26) |
  | `screenshot.tool` | `Text` | `"auto"` | `auto` (cosmic-screenshot), `off`, or a command line run by `/bin/sh -c` with the kind as `$1` and the folder as `$2` | same; F406 | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.top_left` | `CornerAction::{None,Launcher,NotificationCenter,ControlCenter,ShowDesktop,Lock,Workspaces,Command}` | `None` | what the corner does; `None` spawns no surface | `20-SURFACES.md#1-16-hot-corners-spec-integrated-experience`; F409 | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.top_right` | `CornerAction` | `None` | as `top_left` | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.bottom_left` | `CornerAction` | `None` | as `top_left` | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.bottom_right` | `CornerAction` | `None` | as `top_left` (macOS ships Quick Note here; sill has none until M12) | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.top_left_command` | `Text` | `""` | run by `/bin/sh -c` when the corner is `Command` | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.top_right_command` | `Text` | `""` | as `top_left_command` | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.bottom_left_command` | `Text` | `""` | as `top_left_command` | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.bottom_right_command` | `Text` | `""` | as `top_left_command` | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.dwell_ms` | `Ms` | `150` | `0..=2000`; how long the pointer rests before the corner acts | `20-SURFACES.md#3-open-decisions` item 7; F409 | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.rearm_ms` | `Ms` | `500` | `0..=5000`; acts again only after the pointer has left and this long | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.modifier` | `CornerModifier::{None,Super,Alt,Ctrl,Shift}` | `None` | a key held for the corner to act (macOS's modifier option); see G121 | same | proposed (M10 freeze, 2026-09-26) |
  | `hot_corners.size_px` | `Px` | `2` | `1..=8`; the invisible square's side | `20-SURFACES.md#1-16-hot-corners-spec-integrated-experience` (2 x 2 px) | proposed (M10 freeze, 2026-09-26) |

  All Advanced. Pages: `widgets` and `hot_corners` Dock (macOS's "Desktop & Dock"), `calendar`
  Notifications, `screenshot` Keyboard and Shortcuts. design/20 §1.12's row becomes "a widget in
  the notification center (M6's center replaced the clock's popup)", and §1.14's layer row "one
  `Bottom` surface per output, the widgets on a grid inside it".
- **G120 (shell-host; closed by shell-host F60, adopted in "Adopting the M10 parts")** A drag source: `SurfaceHandle::start_drag(mimes, icon)` (named in
  shell-host F35 as apart from G4's drop side): a `wl_data_source` offering `text/uri-list`,
  `start_drag` with the press's serial and an icon surface, and `dnd_finished` or `cancelled`
  back. The thumbnail's drag out waits on it (`start_file_drag` answers `Unsupported`).
- **G121 (shell-host, Wayland)** Modifiers over an unfocused surface: a layer surface with
  keyboard `None` is never sent `wl_keyboard.modifiers` (they go to the focused surface only),
  so a hot corner cannot know whether Super is held as the pointer arrives. `hot_corners.modifier`
  is frozen and defaults to `None`; lane e confirms on KWin. A fix wants a compositor-side
  modifier state (no protocol offers one) or a surface that has the keyboard.

## M10 lane b follow-ups (2026-09-26)

- F440 New desktop widgets fill from the top right (the freeze's proposal, kept); one placements
  file for all outputs; the grid is measured once when the layer mounts, so an output resize does
  not re-place widgets yet.
- F441 The notification center's whole column scrolls (history, then the widgets under it), as
  macOS's Notification Center does; the history list no longer scrolls on its own.
- F442 Now Playing's layout rules are repeated in `widget_now_playing.css`, scoped to the widget,
  because hosts do not load `control_center.css`; a shared file later.
- F443 Not done: Sonoma's dimmed look for desktop widgets under a focused window (optional);
  moving widgets by drag (F413). Q182 (widget frame, sizes, bump) and Q183 (clock face, battery
  ring) wait on quire.

## M10 lane a: calendar (2026-09-26)

- F420 iCalendar is parsed by hand (unfolding with LF/CRLF/BOM, quoted parameters, VEVENT only,
  DATE/UTC/TZID/floating times, vendor-prefixed TZIDs resolve, Windows zone names float,
  DURATION, EXDATE, STATUS, RECURRENCE-ID overrides). Recurrence covers FREQ daily to yearly,
  INTERVAL, COUNT, UNTIL, BYDAY with ordinals, BYMONTHDAY with negatives, BYMONTH, WKST;
  anything else shows once and logs once.
- F421 Two readings of the frozen tests: `occurrences` reads the rule parts it knows back out of
  `Rule.unexpanded` (the parse test keeps BYMONTHDAY there); a tab continuation unfolds to one
  space.
- F422 Sources: a file is one calendar; a vdir directory is one, plus one per subdirectory; the
  live task re-reads on a change (150 ms debounce), a Refresh, or the day turning.
- Q190 (quire; closed by quire 42ce1eb, F527) `MonthGrid` needs a compact size for a small (164 px) desktop cell; styling
  `.ds-month-*` from sill fails DsInternals. Meanwhile the desktop Calendar is macOS's small date
  face (weekday, large date, month) and the center keeps the full month.
- G130 (closed, "Adopting the M10 parts") `notification_center_render`'s click tests flaked once under the full parallel run (60 ms
  advance per click, now with real widgets drawn); passed alone and on rerun.

## Adopting the M10 parts (2026-09-26)

quire's widget parts (design/04 §40) and screenshot thumbnail (§41), quire 42ce1eb's
`MonthDensity`, and shell-host's drag source (its F60) replace sill's stand-ins; Q181, Q182,
Q183, Q190, G120 (for sill) and G130 close.

- F520 **Widgets are quire's `WidgetFrame` in both hosts.** sill's `WidgetHost`/`WidgetSize`
  are quire's (re-exported from `crate::widgets`; `Center` is `WidgetHost::Tile`); `frame.rs`
  keeps only each kind's title row (`frame_title`) and `widget_frame.css` only the quiet line.
  The frame's `id` is `widget_element_id`, so the desktop layer's input and blur regions find
  `div.ds-widget` as before. The desktop grid's wrapper writes `WidgetMetrics { cell, gap }`
  from `widgets.desktop_cell_px`/`desktop_gap_px` (`style_attr()`), so a card is exactly its
  place (the 200 px cell test measures the frame). Footprints are fixed by quire (a medium tile
  is 344 x 164), so the center's sizes changed: the Calendar tile is **Large** (its month does
  not fit one cell's height), every other tile Medium; the column centres them (384 less the
  panel's 12 px padding leaves 360 for 344: checked by a render test and live). On the desktop
  the World Clock became Medium (up to four dials side by side, macOS's medium clock).
- F521 **World Clock is `ClockFace`s**: `Analog` on the desktop, `Digital` in the tile, the
  zone's hour giving `DayPhase` (day 06:00-17:59, `phase_of`), seconds hidden, the city as the
  label and the day and offset quietly under it (`MAX_CITIES` 4). **Battery is `LevelRing`s**
  (permille level, `RingMark::Charging` while charging, the device glyph inside) over a
  `Bumped` percentage; the ring bumps on its own and a render test sees one bump on a level
  change and rest after it. The digital clock bumps on the minute inside quire; the analog dial
  moves its hands and does not bump.
- F522 **The thumbnail is quire's `ShotThumbnail` in the `BannerStack { BottomRight }` row.**
  `card.rs` is the props (Delete as the one `ThumbAction`, `onopen`, `ondrag`, `onhover` feeding
  the hold machine, `id: "thumb"`, `Swipe::Dismiss` with the notifications' `swipe_metrics`);
  `gesture.rs` and the card CSS are gone. Two exits: after a swipe the row is unlisted and the
  stack flies it out along the swipe (quire's `Carried` flight) and its `on_hidden` hides;
  otherwise the row stays listed, the card's `shown` turns `Hidden` and its own `ShotOut`
  plays and hides (`ThumbState::Leaving { exit: ThumbExit::{Card, Row} }`, `row_of`). The
  entrance is the row's `banner-in` together with the card's `rise`. The surface is now 3 px
  taller (`THUMB_HEIGHT` 153: quire's 4 px mat round a 16:10 box).
- F523 **`ImageSource::file` works inside a Blitz document**, proved twice: the harness paints
  the preview (`the_picture_is_the_preview_file_at_its_own_shape` compares the painted
  picture's mean colour with the fixture's: (172, 188, 207) against (166, 184, 205), where the
  card's own plate is a light grey), and live on the nested cosmic-comp
  (`dev/out/screenshot-thumb-cosmic.png`). No `png(bytes)` fallback was needed. `ImageSize`
  comes from the PNG's IHDR (`picture::png_size`; anything else is drawn 16:10).
- F524 **Drag out (G120).** `ondrag` calls `Thumb::drag_out`: `ThumbIn::DragStarted` (the hold
  pauses, `Pointer::Dragging`), then a task of the thumbnail's scope awaits
  `SurfaceHandle::start_drag(DragSpec { Uris([file URI]), CopyOrMove, DragIcon::Root {
  ShotGhost, 120 x 78 } })`; the ghost's document carries the thumbnail's appearance as props
  (`GhostLook`), since the icon surface has none of the root contexts. The outcome maps
  (`drag_end`) to `Dropped{Copy}` → leave `Dragged` (file kept, card dismissed),
  `Dropped{Move}` → leave `Moved` (new `ShotLeave::Moved`: the service drops it from `Shots`
  and touches the file no more), `Cancelled` or `Unsupported` → back to the hold with the rest
  (`rest_ms`), the card where it was; every outcome is logged (`sill: screenshot drag: ..`).
  With no host (a harness) the drag is refused at once and the hold resumes.
- F525 **Drag acceptance** (`dev/accept-screenshot-drag.sh`, headless KWin 1920 x 1080, one
  fake-input client, `dock.position = "left"` so the thumbnail sits at the true corner): foot
  is not installed, so the run records **Cancelled over nothing**: pressed at (1760, 963) on the
  picture, 12 px left past quire's threshold, glided to (700, 400), released: the daemon logged
  `screenshot drag: Cancelled`, the card was still up after, the file was still there, and the
  thumbnail unmapped 1246 ms after the release (the 1500 ms rest from the drag's end). A drop on
  a real target (`Dropped{Copy|Move}`) is table-tested only here; shell-host F60 proved the
  drop path against foot.
- F526 **G130.** `center_support::Running::{until, commands_until}` advance in 20 ms steps
  until what the test checks holds (rows opened, a command sent, the hover shown, the panel
  leaving), at most 2 s; the click tests no longer rely on a 60 ms advance. The one negative
  check (a press inside keeps it open) waits a fixed 400 ms.
- F527 **Q190: the desktop Calendar is quire's compact month.** The small date face and its CSS
  are gone; the Calendar draws `MonthGrid` in both hosts with the default `MonthDensity::Auto`,
  which the enclosing `WidgetFrame` resolves: compact in the desktop's Small frame, regular in
  the center's Large tile (`data-density`). A render test steps through twelve months (a
  six-week one among them) and checks each stays inside the Small frame's 132 x 132 content
  box at the default `widgets.desktop_cell_px` 164; live on the nested cosmic-comp in
  `widgets-desktop-cosmic.png`. **A cell under 164 overflows**: the compact six-week month is
  126 x 132, exactly the content box at 164, and quire's grid does not shrink further (the
  key's range goes down to 120).

### Acceptance (nested cosmic-comp, 2560 x 1600)

- `dev/accept-widgets.sh`: PASS all six (3 tiles, then 5, 0 frames idle, the desktop layer's
  two frames at 2380 16 2543 181 and 2380 196 2543 361, 0 frames idle, the toggle back). The
  tile detector now probes 24 px in, since quire's tiles are centred 20 px in.
- `dev/accept-calendar.sh`: PASS all four.
- `dev/accept-screenshot.sh`: PASS all five (shown 131 ms after the debug shot, hidden at
  5724 ms with the file kept; the tool's shot shown in 110 ms and a click ran the opener; a
  hover kept it and it left 1849 ms after the leave; 0 frames hidden).
- Screenshots retaken: `widgets-center-cosmic.png`, `widgets-center-all-cosmic.png`,
  `widgets-desktop-cosmic.png`, `screenshot-thumb-cosmic.png` (and the calendar's two).

### Left

- The center's tiles are fixed at 344 wide: a `notifications.center_width_px` under 368 would
  overflow them (quire's footprint is not relative to the column).
- The Calendar's Large tile shows empty room under a five-week month.
- No live picture of the analog World Clock or the battery rings: the acceptance's private bus
  has no batteries and the desktop layer's default widgets are Calendar and Battery.
- A drop onto a real target from sill (foot or a file manager) is still to be run.

## hot_corners.modifier retired (2026-09-26)

- The key, `CornerModifier`, and its design/22 row (quire 42ce1eb) are gone: a keyboard-less
  layer surface has no portable source for held modifiers (shell-host F61). The wish stays in
  docs/cosmic-gaps.md as a fork item. An existing settings.toml line `modifier = ...` under
  `[hot_corners]` is kept in `extra` and ignored.

## Scroll engine (branch `scroll-wire`, 2026-09-26)

- F528 **sill's own `scroll` and `scrollbar` sections drive shell-host's engine
  (`HostCtx::set_scroll_config`, shell-host FINDINGS F63).** `crates/sill/src/scroll.rs`
  (new) maps `sill_settings::ScrollSettings` / `ScrollbarSettings` (design/22 §3.7 / §3.8) to
  shell-host's `scroll::config::ScrollConfig` field by field: the two sides carry the same
  keys under the same serde spelling but as two crates' own newtypes and enums (`Px`, `Ms`,
  `Fraction`, `Scalar`; `NaturalScroll`, `Momentum`, `MomentumModel`, `RubberBand`,
  `ScrollbarVisibility`, `TrackClick`), so a shared type was not available and the mapping is a
  pure `fn config(&ShellFile) -> ScrollConfig`, table-tested (one row per enum this module
  converts, plus a numeric field and the whole-default equality). `daemon.rs` calls it in three
  places, all through the same function: `HostEvent::Started` (using the settings channel's
  already-initial value, since `ctx` exists only inside `host.run`'s closure), every settled
  `HostEvent::App(ShellMsg::Settings(file))` (the watcher), and `Request::Reload` (`sill
  reload`, which only republishes the settings channel and previously touched nothing the host
  reads — `resize_bars` has the same gap, pre-existing and out of scope here). `on_event`'s
  frozen signature (F36) is respected: the Started call sits in `run_event`, not `on_event`,
  since it needs the settings sender; the Settings-changed call sits in `on_event` beside
  `resize_bars`, since it needs only the event's own payload.
- F529 **`.sill-nc-column` carries `data-overscroll="band"`** (quire design/11 §11.3.7's
  eligibility table): the whole notification-center column is one scroller (title, history and
  the widgets slot together, notification_center.css's own comment), and it rubber-bands as
  macOS's Notification Center does. The launcher's result list (Spotlight does not band) and
  every menu (never band) carry nothing, unchanged.
- F530 **dev/accept-real-input.sh's `sill` calls reached the ambient session, not the headless
  KWin.** Every call (`ping`, `debug capture`, `debug stats`, `power-menu open/close`, and the
  bare `"$SILL_BIN" quit` in `stop_daemon`, which bypassed the `sill()` wrapper entirely) ran
  with whatever `WAYLAND_DISPLAY`/`DBUS_SESSION_BUS_ADDRESS` this shell already had, not the
  private headless one `start_daemon`'s subshell sources for the daemon itself. Fixed to match
  dev/accept-notifications-real-input.sh's own pattern: `start_kwin` now parses `.kwin-env`'s
  socket and bus into `WL`/`BUS` without sourcing the file into this shell (so
  `kwin-virtual.sh stop`'s bus-matching kill never catches this script), an `in_env` helper
  exports them for one command, `sill()` wraps every CLI call in it, and `stop_daemon`'s `quit`
  goes through `in_env` too. `XDG_RUNTIME_DIR` is unchanged (both scripts share the ambient
  one), so this was latent rather than a live failure; re-run clean, all six steps PASS.
- F531 **Live, on shell-host's headless KWin (`scripts/dev/kwin-virtual.sh`,
  `fake-input.sh`'s `axis vertical VALUE`, one client): the engine scrolls the column.** A new
  step in dev/accept-notifications-real-input.sh (`step_center_scroll`, 10th): 15 notifications
  under distinct app names overflow `.sill-nc-column` (History is live the moment each arrives,
  independent of the banners' own hold, so no wait is needed before opening the center); five
  real wheel detents over the panel (`axis vertical 15`, KWin's fake input sends a source-less
  frame, which the engine treats as wheel per shell-host F63's Sources rule) change a hash of
  the capture's top band (title included, since the whole column scrolls as one): `9d7664f3…`
  to `279e1252…`. **Rubber-band at the top is not exercised, and is not exercisable this way**:
  design/11 §11.3.7's last row is "wheel, keyboard, programmatic: never stretch; clamp", and
  shell-host's own test 12 checks exactly this ("no stretch at an edge"); only a finger/touch
  gesture stretches, and fake-input has no finger source (shell-host F63's own live gap,
  unchanged by this branch). `use_scroll`'s `ScrollState` has no reader outside a Dioxus
  component (`SurfaceHandle::scroll_board` is `pub(crate)` to shell-host), and adding one is a
  shell-host API change outside this task, so the rubber-band claim is recorded, not proven,
  per the task's own fallback. `step_idle` right after (5 s, 0 frames on both surfaces) shows
  the engine settles cleanly after the wheel gesture too.
- **Acceptance, headless KWin 1920x1080, real input:** `dev/accept-real-input.sh`: PASS all six
  (`step_cc_click_escape`, `step_cc_outside_click`, `step_menu_keyboard`, `step_menu_arrows`,
  `step_menu_escape`, `step_menu_scrim_click`). `dev/accept-notifications-real-input.sh`: PASS
  all twelve, `step_center_scroll` (new) among them. Both stopped the headless KWin on exit;
  wayland-0 untouched throughout.
- **Not done, or unrelated.** `cargo test --workspace --features sill/debug` has one unrelated
  failure, `sill-surfaces::calendar_render::on_the_desktop_the_calendar_is_the_compact_month_
  inside_its_small_frame`: it renders quire's compact `MonthGrid` against real wall-clock
  `Zoned::now()` (F527: "exactly the content box at 164 px", a knife's-edge fit), and the sibling
  quire checkout's `MonthGrid` now lays a six-week month out at 126 x 114 rather than F527's
  126 x 132, missing the frame's top padding by 3.5 px. No file this branch touches is on the
  path to that component; not investigated further (quire is a sibling repo, out of this
  task's scope, and shared with other concurrent work).

## M11 freeze (2026-09-26)

sill's own lock screen, polkit prompt and app switcher (design/20 §1.9, §1.10, §1.11; design/13
§13.3.5), a choice (`sill`) beside M7's borrowed locker and polkit agent, frozen on
`m11-freeze` so three agents can fill them in parallel and a fourth can prove them under real
input. Everything compiles; every pure function a lane fills is `todo!()` with its table test
`#[ignore = "M11 fill: <lane>"]` (30: 16 a-lock, 9 b-polkit, 5 c-switcher), every component
draws nothing, both new services' `live::run` return at once, and every daemon follower a lane
fills is a no-op, so the suite stays green and `sill run` starts. Findings are numbered from
F540/Q240/G180 (F520-F531 went to the scroll engine); the lanes take F560/Q250/G190 (a),
F580/Q260/G200 (b), F600/Q270/G210 (c) and F620/Q280/G220 (d).

Both upstreams landed while the freeze ran, and the freeze is written against their real
shapes, not the announced ones: shell-host's session lock (its F65, on its master) and quire's
M11 parts (quire master `89f0e37`, CONSUMING "Lock, polkit and switcher").

Arc heuristics (quire design/09 §5): the lock screen and the polkit prompt serve none of the
seven directly, so they are chrome and stay as small and conventional as macOS's (09 §5), with
every choice a key (H6: `session.locker`, `session.polkit_agent`, `session.lock_clock`); the
switcher serves H5 (fewer icons, more shortcuts: Cmd+Tab, then Q and H on the selection, is
the habit it teaches).

### Research

- **F540 How a locker checks a password without root.** PAM's `pam_unix`, run by a process that
  is not root, calls the setuid helper `unix_chkpwd` (here `/usr/sbin/unix_chkpwd`, mode
  `rwsr-xr-x`), which checks only the calling user's own password against `/etc/shadow`
  (`unix_chkpwd(8)`); a user-level locker needs nothing but a PAM service file. The lockers of
  record: **swaylock** (MIT) forks a child first and runs PAM there, talking to it over a pipe
  (`pam.c`, `comm.c`; service `swaylock`, whose file is `auth include login`); **hyprlock**
  (BSD-3) runs PAM on a thread of its own (service `hyprlock`); **cosmic-greeter**'s locker
  (GPL-3.0, read at `epoch-1.8.0` for facts only) calls `pam-client` on tokio's blocking pool
  with service `cosmic-greeter`, `authenticate` then `acct_mgmt`. None of their PAM files is
  installed here (Fedora 44 has `login`, `system-auth`, `password-auth`, `kde`, `kscreensaver`).
  **sill takes swaylock's shape**: `sill_services::auth::PamHelper` runs `sill auth-check` (a
  hidden subcommand, `crates/sill/src/auth_check.rs`) once per attempt, writes the service, the
  user and the secret NUL-terminated on its stdin, and reads `info`/`error` lines and one of
  `accepted`/`refused`/`fault WORD` back (`auth::protocol`). The child calls PAM
  (`auth::pam::check`: `authenticate` then `account_management`; the first hidden prompt gets
  the secret, a later prompt a conversation error). A module that hangs (a network directory, a
  fingerprint reader) is killed with its child after `CHECK_TIMEOUT` (15 s) instead of wedging a
  thread, and the daemon never holds a PAM handle. The service is `sill` when a file for it
  exists in `/etc/pam.d`, `/usr/lib/pam.d` or `/usr/etc/pam.d` (`dist/pam.d/sill`: `auth include
  login`, `account include login`, swaylock's shape), else `login` (`auth::pam::service`).
- **F541 Dependencies: `nonstick` 0.1.2 (MIT) added** (sill-services), with `libpam-sys`,
  `libpam-sys-impls` and `libpam-sys-helpers` 0.2 (all MIT): a safe API (`TransactionBuilder`,
  `ConversationAdapter`) over pre-generated bindings, so it builds without system headers or
  libclang (0.8 s in a scratch crate here). Rejected, each tried in a scratch crate on this
  machine: `pam` 0.8.0 (MIT/Apache) and `pam-client` 0.5.0 (MPL-2.0, cosmic-greeter's) both need
  `pam-sys` 1.0.0-alpha5, whose build script runs bindgen and fails with no `libclang.so`; `pam`
  also brings `users` 0.11, unmaintained. No polkit crate: the agent is `zbus` 5 (pinned) plus
  polkit's own helper process (F545); libpolkit-agent-1 would need FFI, which `unsafe_code =
  "deny"` rules out. `cargo deny check licenses`: `licenses ok`; the boundary holds.
- **F545 How a polkit agent authenticates.** An agent never checks a password and never needs
  root: it passes the request's cookie to `polkit-agent-helper-1` (setuid root; `/usr/lib/
  polkit-1/` on Fedora and Arch, `/usr/libexec/` on Debian and Ubuntu), which runs PAM as the
  chosen identity and on success tells polkitd itself (`AuthenticationAgentResponse2`), so an
  agent cannot forge an answer. libpolkit-agent-1's `PolkitAgentSession` (under polkit-gnome,
  polkit-kde and lxqt-policykit; LGPL, not linked) and cosmic-osd's own Rust version
  (`subscriptions/polkit_agent_helper.rs`, GPL-3.0, facts only) speak one line protocol: with
  polkit 124+'s socket-activated helper (`/run/polkit/agent-helper.socket`) the agent connects
  and writes the user name and the cookie, a line each; otherwise it runs the setuid helper with
  the user name as its argument and writes the cookie on stdin. The helper writes
  `PAM_PROMPT_ECHO_OFF <prompt>`, `PAM_PROMPT_ECHO_ON <prompt>`, `PAM_ERROR_MSG <text>`,
  `PAM_TEXT_INFO <text>`, then `SUCCESS` or `FAILURE`; a prompt is answered with one line.
  cosmic-osd retries a request three times. On this machine: polkit 127 (`rpm -q polkit`), the
  setuid helper at `/usr/lib/polkit-1/polkit-agent-helper-1`, and `polkit-agent-helper.socket`
  installed but disabled and inactive (its service runs `polkit-agent-helper-1
  --socket-activated` on the socket), so the setuid helper is the route. The identity asked for
  is the session's own user when listed, else root, else the first `unix-user` (GNOME Shell's
  and cosmic-osd's rule). Registration: `org.freedesktop.PolicyKit1.Authority.
  RegisterAuthenticationAgent(("unix-session", {"session-id": <logind id>}), locale,
  object_path)`; the agent object is `org.freedesktop.PolicyKit1.AuthenticationAgent` with
  `BeginAuthentication(s action_id, s message, s icon_name, a{ss} details, s cookie, a(sa{sv})
  identities)`, answered when the request completes (errors `org.freedesktop.PolicyKit1.Error.
  Cancelled` / `.Failed`), and `CancelAuthentication(s cookie)`.
- **F549 How Cmd+Tab reaches sill.** COSMIC's shortcut defaults (`/usr/share/cosmic/
  com.system76.CosmicSettings.Shortcuts/v1/defaults`) bind `Alt+Tab` and `Super+Tab` to
  `System(WindowSwitcher)` and both with Shift to `WindowSwitcherPrevious`; `system_actions`
  runs `cosmic-launcher alt-tab` / `shift-alt-tab` for them. cosmic-comp has no switcher code of
  its own (its 1.0 source has "switcher" only in the action names): the action spawns the
  command, and cosmic-launcher sees the release because its layer surface takes the keyboard
  (`Exclusive`) and reads the modifiers. Toshy maps physical Cmd+Tab to `Alt+Tab` with `bind`
  (`~/.config/toshy/toshy_config.py`: `C("RC-Tab"): [getDK(), bind, C("Alt-Tab"), setDK(None)]`,
  `Shift-RC-Tab` likewise), which keeps Alt held for as long as Cmd is, so the compositor sees
  Alt+Tab and the release as Alt going up. So `dist/cosmic`'s `system_actions` gains
  `WindowSwitcher: "sill switcher next"` and `WindowSwitcherPrevious: "sill switcher prev"`
  (lane c); the switcher surface maps at the chord with keyboard `Exclusive` (before the panel
  shows) and reads the release as a key-up of `Alt` or `Meta` (`surfaces::switcher::keys::
  chord_after_keyup`; a key-up's modifiers are still the state before it, since
  `wl_keyboard.modifiers` follows the key). A release before the surface has the keyboard (a
  quick tap racing the `sill switcher next` process, the IPC and the map) is never a key event:
  G180.

### What is frozen

- **F542 The auth service** (`sill_services::auth`, `Services.auth` / `auth_control`). `Auth {
  account, phase, failures, notes }`, `AuthPhase::{Idle, Checking, Wrong, LockedOut { until:
  UnixMs }, Accepted, Unavailable(AuthFault)}`, `AuthCommand::{Submit(Secret), Reset,
  Fake(FakeVerdict)}`. `Secret` never prints (`Secret(..)`), is never serialised, and zeroes its
  bytes on drop; the helper's request bytes are zeroed after writing and after reading. The seam
  is `trait Authenticator { fn check(&self, &Attempt) -> Verdict }` (blocking, on the blocking
  pool) with `PamHelper` and `FakeAuthenticator { rule: FakeRule::{Accepts(text), RefusesAll,
  Fails(fault)}, delay }` (implemented); `spawn` uses `PamHelper::this_exe()`, `spawn_with` any
  authenticator. The pure `lockout` machine: a submit while checking or waiting is `Busy` (a held
  Return queues nothing); `LockoutPolicy::DEFAULT` is five free tries, then 60 s, doubling to
  15 min (macOS's "try again in 1 minute"); `Unavailable` is never counted; a new lock (`Reset`)
  keeps the count and never shortens a wait. **The numbers are constants, not settings**,
  deliberately (a lockout is a security decision; flagged for the user). Nothing runs PAM
  before a `Submit`, so `Services::spawn` in a test checks nothing.
- **F543 Caps Lock** comes from the lock surface's own key events: shell-host's
  `modifiers_from` sets `Modifiers::CAPS_LOCK` from xkb's locked state, and a lock surface
  always has the keyboard (`surfaces::lock::view::caps_lock` into quire's `CapsLock`). The state
  before the first key is not known (G180).
- **F544 The own lock** (`session.locker = "sill"`). `LockerChoice::Own` resolves to
  `LockerMode::Own { triggers }` (logind followed only in a sill session, as for a program),
  which the machine sees as `Runner::Runs`: the M7 machine is unchanged, and the sleep delay and
  the logind triggers work as for a program. `Lock` gains `drawer: LockDrawer::{Program, Shell}`;
  with `Shell` a `Spawn` starts nothing and the daemon follows the phase onto shell-host's
  session lock, answering with `LockerCommand::Shell(ShellLockEvent::{Locked, Unlocked,
  Finished, Refused})` (`Locked` is the machine's `Reported`, the rest `Exited`). M7's borrowed
  paths are untouched. The daemon side (`crates/sill/src/lock/`): `lock_wish(&Lock, &Auth)`
  (held while the drawer is `Shell` and the phase is not `Unlocked`, until `Accepted`);
  `spawn_follower` sends `ShellMsg::Lock(wish)`; `apply` resets the auth service, points the
  `LockLink` at the main display and calls `lock_session` with a `LockFactory` of `LockScreen`s
  (`daemon::lock_factory`: `surface_root` plus the link); `on_host_event` turns `HostEvent::Lock`
  into `ShellLockEvent`s and moves the prompt to the lock surface that gains the keyboard. **The
  unlock is the lock surface's**: on `Accepted` every lock surface fades (`--t-move --e-exit`)
  and the prompt's calls `shell_host::use_lock().unlock()` when its fade settles; the daemon's
  `unlock_session` is a backstop after `UNLOCK_BACKSTOP_MS` (1.5 s). The surface
  (`sill_surfaces::surfaces::lock`, one per output): quire's `LockScreen { wallpaper, clock,
  prompt }` in `Ds { Window, Viewport }` with `LockClock` (`clock_lines`, `session.lock_clock`)
  and, on the `LockLink` output, `LockPrompt` (`lock_user`, `prompt_state`: `LockedOut`'s
  `until` worded as the lock clock words a time, `caps_lock`); `onsubmit`'s text becomes a
  `Secret` at once and is never kept in a signal. The backdrop is a blurred copy the wallpaper
  service writes beside each prepared image (`wallpaper::backdrop_file`:
  `<hash>-<w>x<h>-backdrop.png`, `ensure_backdrop`), since Blitz blurs nothing. **Re-lock after
  a crash**: a compositor keeps the session locked when its locker dies (F381), and
  `dist/sill-session` restarts `sill run`; no protocol says whether a session is locked, so the
  service keeps `$XDG_RUNTIME_DIR/sill/locked` while its own lock is `Locked`, and a starting
  service in `Own` mode that finds it locks at once (`locker::marker`; a stale marker costs one
  password prompt, never an open session). **Recommendation: keep `auto` (the borrowed locker)
  as the default** until lane a's acceptance passes on the nested cosmic-comp and the user has
  done the "Needs a sill login" checks (a real password, lock before sleep, a crash while
  locked); then make `auto` prefer `sill` wherever `ext-session-lock-v1` is present. A lock
  screen that fails open, or locks its person out, costs more than a borrowed one that looks
  different.
- **F546 The own polkit agent** (`sill_services::polkit_agent`, `Services.polkit` /
  `polkit_control`), chosen by `session.polkit_agent = "sill"` (`AgentChoice::Own`; `sill
  polkit-agent` then execs nothing). **Passive until the daemon's first
  `PolkitCommand::Policy`** (`crates/sill/src/polkit.rs` `spawn_policy`: `Own` exactly for
  `sill`), so a test never touches the system bus. `Setup { bus: PolkitBus::{System, Address},
  helper: HelperChoice::{System, Use(route)}, session: SessionChoice::{Ours, Id} }` lets every
  live test run on a private bus with a stand-in Authority and a stand-in helper. Pure parts:
  `pick_identity`, `helper_route`, `opening`, `parse_helper_line`, and the reducer
  `reduce(PolkitState, PolkitEvent) -> (PolkitState, Vec<PolkitEffect>)` (one request on screen,
  the rest queued; `MAX_TRIES` 3 helper runs; `Completion::{Authorized, Cancelled, Failed}`),
  whose rules are its module doc. `dbus.rs` holds the `#[interface]` object, the `DBusError`
  errors and the Authority `#[proxy]`, so the macros are proven to build; `BeginAuthentication`
  waits on a oneshot until its request completes. The prompt (`surfaces::polkit_prompt`,
  `sill-polkit-<out>`): one warm hidden Overlay per output (full-output, keyboard `Exclusive`,
  input `Whole`, blur on the sheet), shown on the banners' output while a request is current,
  drawing quire's `PolkitPrompt` from `prompt_view` (`Starting` and `Checking` are quire's
  `Checking`, `Asking` is `Idle` or `Wrong` right after a failed try, `detail` the action id and
  polkitd's details); Enter, Escape and the scrim are quire's. Nested under KDE the daemon shares
  KDE's logind session, whose agent (polkit-kde) holds it, so polkitd refuses sill's
  (`Registration::Refused`), which is correct; only a sill login registers for real.
- **F547 No test reaches polkitd, the system bus, or PAM with a real password.** Live tests use
  `tests/support/private_bus.rs` (F264) with lane b's stand-in Authority (it records the subject
  and calls `BeginAuthentication` on sill's agent) and stand-in helper (a program speaking F545's
  protocol that accepts one fixed secret); the auth tests use `FakeAuthenticator`. The one live
  PAM test lane a may add runs only when `SILL_PAM_TEST_USER` and `SILL_PAM_TEST_SECRET` name a
  throwaway account and is skipped, saying so, otherwise; this machine has no such account.
  `sill debug auth accept` is refused by a daemon built without `debug` (`Unsupported`: an IPC
  request must never unlock a real session), and scripts run a debug daemon only on the nested
  compositor.
- **F548 The switcher.** `sill_services::switcher` is pure (no task: the switcher keeps no state
  outside the surface's machine): `switcher_apps(&Toplevels, &AppIndex) -> Vec<SwitcherApp { app,
  windows, shown: AppShown::{Visible, AllMinimized} }>` groups the MRU toplevels by app
  (`match_app`, else the raw `app_id`); `activate`, `quit`, `hide` turn a verb into
  `WlCommand`s. The surface (`surfaces::switcher`): the frozen M4-era `machine` (show delay, quick
  tap, Tab, grave, arrows, Q, H, Escape, hover, click) now drives `SwitcherPanel { output }`, one
  warm hidden Overlay per output (`switcher_layer`: full-output, keyboard `Exclusive`, input and
  blur on the panel only, radius 18); `SwitcherLink` carries numbered `SwitcherAsk::{Chord(Dir),
  Cancel, Commit, Fake(apps)}` from the daemon (implemented); `keys::{sw_key,
  chord_after_keyup}` read keys and the release; `tiles::{tiles, metrics_of, key_of, index_of,
  output_width}` feed quire's `AppSwitcher` (keyed by `AppKey`, which `index_of` turns back into
  the machine's index). It opens on the banners' output (the pointer's output is not known to the
  daemon, F140). The show delay's `Tick` is driven by quire's timer (rule 4);
  `switcher.quick_tap_ms` is registered but the machine treats any release before the show delay
  as the quick tap (Q242).
- **F550 quire's M11 parts** landed on quire master (`89f0e37`) during the freeze, and sill uses
  them directly: `ds::{LockScreen, LockClock, LockPrompt, LockUser, PromptState, CapsLock,
  LockLook, PolkitPrompt, AppSwitcher, SwitcherApp, AppKey, TilePresence, SwitcherMetrics,
  switcher_fit}`. sill's view functions return quire's own types (`PromptState`, `CapsLock`,
  `LockUser`, quire's `SwitcherApp` tiles), so no adapter file is needed. Styling nothing under
  `.ds-lock*`, `.ds-polkit*`, `.ds-switcher*`. quire's `LockUser` carries an initials face only
  (Q243).
- **F551 shell-host's session lock** landed on shell-host's master (its F65) during the freeze;
  `crates/sill/src/lock/host.rs` is written against it: `HostCtx::lock_session(LockConfig {
  namespace: "sill-lock" }, factory: Fn(OutputInfo) -> Root) -> Result<LockId,
  LockError::{Unsupported, InProgress}>`, `unlock_session(LockId)`, `lock_surfaces(LockId)`,
  `HostEvent::Lock { id, state: LockState::{Locked, Unlocked, Finished} }`, and in documents
  `use_lock()` (`unlock()` sends `unlock_and_destroy` at the next flush) and `use_lock_phase()`.
  `SessionLockHost` is the seam the daemon's lock logic is written against, so its table tests
  use a fake host. KWin 6.7.5 has no `ext-session-lock-v1` (`Unsupported`); cosmic-comp 1.8
  locks. `on_event` gains the `HostEvent::Lock` arm (answered in `run_event`); sill master's
  coming no-op arm (branch `host-lock-arm`) merges into it.
- **F552 IPC and CLI** (`sill-ipc/src/session.rs`, `crates/sill/src/cli_session.rs`, wire lines
  pinned in `sill-ipc/tests/serde.rs`): `Request::Switcher(SwitcherRequest::{Next, Prev, Cancel,
  Commit})` (`sill switcher next|prev|cancel|commit`); `Request::LockStatus(Status)` (`sill lock
  status`, answered `Response::Lock(LockReport { phase, drawer })`; `sill lock` alone is `sill
  lock now`, M7's `Request::Lock`); `Request::Polkit(Status)` (`sill polkit status`,
  `Response::Polkit(PolkitReport { registration, showing, waiting })`);
  `DebugRequest::Auth(FakeAuth::{Accept, Refuse})` (`sill debug auth accept|refuse`);
  `DebugRequest::Polkit(FakePolkit { action_id, message })` (`sill debug polkit [--action]
  [--message]`; its stand-in helper accepts `sill-fake`); `DebugRequest::Switcher(FakeSwitcher {
  apps })` (`sill debug switcher APP...`: opens at once on made-up apps whose Q, H and release
  touch no window); the hidden `sill auth-check`. Daemon: `ShellMsg::Lock(LockWish)`,
  `Links.{switcher, lock}` (in every root), `OutputSurfaces.{polkit_prompt, switcher}` (in `debug
  stats`); hooks `use_auth`, `use_polkit`, `use_locker`.
- **F553 Settings.** `session.locker` and `session.polkit_agent` gain the word `sill` (their help
  text says so; their design/22 rows need the alternative, Q241). New: `session.lock_clock`
  (`LockClock::{H24, H12}`, default `H24`, the bar clock's), in `AWAITING_ROWS`. `switcher.*`
  already has its rows (§3.11: `show_delay_ms` 150, `quick_tap_ms`, `icon_size_px` 96,
  `cell_size_px` 112, `cell_gap_px` 8, `overflow_min_icon_px` 48); nothing is added there.
  Considered and not registered: `session.lock_grace_s` (macOS's "require password after sleep
  begins": a lock any key opens within N seconds is a security trade-off for the user to
  decide), and the lockout numbers (F542).

### The M11 fill-in: three agents, disjoint files, then real input

Every agent reads CONVENTIONS §0 and §1 (fill bodies, add private helpers and tests; never
change a frozen signature, public field or variant; stop and report instead), ORCHESTRATION
"Coherence rules", design/20 §1.9-§1.11, design/13 §13.3.5, quire design/09 §5, quire
CONSUMING "Lock, polkit and switcher", and this section. Everyone runs, before reporting:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features sill/debug
./scripts/check-boundary.sh
cargo deny check licenses
```

**Safety, binding on every lane.** Never lock `wayland-0` (the user's KDE session): a script
that locks sets `WAYLAND_DISPLAY=wayland-1` (the nested cosmic-comp), pings it first
(`WAYLAND_DISPLAY=wayland-1 wayland-info >/dev/null`) and **never stops or restarts it**; a lock
left behind is recovered with `WAYLAND_DISPLAY=wayland-1 cargo run -p sill-services --example
session_locker -- 0` (F381). Never register a polkit agent on the real session or system bus
(private bus only, F547). Never call PAM with the user's real password (fakes; the
throwaway-user test only). Never quit or minimise the user's real windows: switcher scripts use
`sill debug switcher` or windows the script starts inside the nested compositor. A script that
starts `sill run` gives it its own `XDG_RUNTIME_DIR` (with `WAYLAND_DISPLAY` as an absolute
socket path) and a scratch `XDG_CONFIG_HOME`, so latchkey's lock and the lock marker are the
script's. Files not listed under an agent are read-only to all: in particular
`sill-services/src/{lib.rs,services.rs}`, `sill-surfaces/src/hooks/**`,
`sill-surfaces/tests/support/fake.rs`, `sill-settings/**`, `sill-ipc/**`, and
`crates/sill/src/{daemon.rs,cli.rs,cli_session.rs,links.rs,outputs.rs,msg.rs,main.rs}`.

**(a) a-lock: the auth service, the own lock and the lock surface** (09 §5 chrome; H6).
- Owns: `crates/sill-services/src/auth/**`; `crates/sill-services/src/locker/**` (the `Own`
  mode and the marker; M7's paths may not change behaviour); `crates/sill-services/src/
  wallpaper/{backdrop.rs,resize.rs}` and the one `ensure_backdrop` call in `wallpaper/mod.rs`'s
  `publish`; new `crates/sill-services/tests/auth_live.rs` and a stand-in helper example (its
  `[[example]]` entry only); `crates/sill-surfaces/src/surfaces/lock/**`; new
  `tests/{lock_render,lock_lint}.rs`; `crates/sill/src/lock/**`; `crates/sill/src/auth_check.rs`;
  `dist/pam.d/sill`; a new `dev/accept-lock.sh`; the lock's entries in `docs/manual-checks.md`.
- Frozen: every `pub` item `sill_services::auth` exports; `LockerChoice::Own`, `LockerMode::Own`,
  `Lock.drawer`, `LockDrawer`, `ShellLockEvent`, `LockerCommand::Shell`, `locker::marker`'s API;
  `wallpaper::{backdrop_file, ensure_backdrop, BACKDROP_SIGMA_AT_1920}`; `surfaces::lock`'s
  exports (`LockScreen { output }`, `LockLink`, `PromptOutput`, `ClockLines`, `caps_lock`,
  `prompt_state`, `lock_user`, `clock_lines`, `backdrop_for`); `crate::lock::{LockWish,
  lock_wish, ShellLock, spawn_follower, apply, on_host_event, status, report_of, fake_auth,
  UNLOCK_BACKSTOP_MS}` and `lock::host`'s API.
- Done: the 16 `a-lock` tests pass un-ignored; `auth::live::run` (its module doc) and
  `PamHelper`; the locker's marker and restore; the backdrop written beside each prepared image;
  the lock surface as `screen.rs`'s doc says; `spawn_follower`, `apply`, `on_host_event` as their
  docs say; the unlock fade and `use_lock().unlock()`.
- Verify: the gates; `cargo test -p sill-services --test auth_live -- --ignored --nocapture`
  (a `FakeAuthenticator` through `spawn_with`: wrong five times then locked out with a
  wall-clock `until`, `Busy` while checking, `Fake`, `Reset`; `PamHelper` against the stand-in
  helper, including one killed at a shortened timeout; the throwaway-user PAM test, skipped here);
  `dev/accept-lock.sh` on the nested cosmic-comp (`wayland-1`, pinged first, never stopped), a
  `--features debug` daemon with `session.locker = "sill"`: (1) `sill lock`, then `sill lock
  status` says `locked`/`shell` within 1 s, and `session_locker 3` (a second lock client) is
  refused while it holds; (2) `sill debug stats` lists `sill-lock/<out>` for every output, and
  a capture of the prompt's surface shows the clock and the field (`dev/out/lock-cosmic.png`);
  (3) `sill debug auth refuse` keeps it locked; (4) `sill debug auth accept` unlocks within the
  fade plus 500 ms (`status` unlocked, no `sill-lock/*` in stats); (5) kill the daemon while
  locked and start it again: it locks at once (the marker), and `sill debug auth accept`
  unlocks; (6) a release build answers `sill debug auth accept` with `unsupported`. Whatever
  fails, the script ends with the recovery lock-and-unlock.

**(b) b-polkit: the agent and its prompt** (09 §5 chrome; H6).
- Owns: `crates/sill-services/src/polkit_agent/**`; `crates/sill-services/tests/polkit_live.rs`;
  new examples `polkit_authority` (a stand-in Authority on a private bus) and `polkit_helper` (a
  stand-in helper) with their `[[example]]` entries; `crates/sill-surfaces/src/surfaces/
  polkit_prompt/**` (the implemented `layer.rs` may change inside, never in signature); new
  `tests/{polkit_prompt_render,polkit_prompt_lint}.rs`; `crates/sill/src/polkit.rs`; a new
  `dev/accept-polkit.sh`; the polkit entries in `docs/manual-checks.md`.
- Frozen: every `pub` item `sill_services::polkit_agent` exports (types, fields, variants,
  `Setup` and its enums, `spawn`, `spawn_with`, `FAKE_SECRET`, `MAX_TRIES`, `AGENT_PATH`),
  `AgentChoice::Own`, `AgentMode::Own`; `PolkitPrompt { output }`, `polkit_layer`, `PANEL_ID`,
  `PolkitPromptView`, `prompt_view`, `detail_of`; `crate::polkit::{policy_of, spawn_policy,
  spawn_missing, namespace, dispatch, report_of, fake}`. `dbus.rs` is private: its D-Bus names
  and signatures are polkit's.
- Done: the 9 `b-polkit` tests pass un-ignored; `live::run` (its module doc) with the helper as a
  `kill_on_drop` child or the socket; the prompt as `panel.rs`'s doc says.
- Verify: the gates; `cargo test -p sill-services --test polkit_live -- --ignored --nocapture`
  on a private bus (F264): (1) `Policy(Own)` registers, and the stand-in Authority saw
  `("unix-session", {"session-id": <the Setup's id>})` and `AGENT_PATH`; (2) a
  `BeginAuthentication` publishes a `PromptView`, and an `Answer` of the stand-in's secret makes
  the call return; (3) three wrong answers make it fail `...Error.Failed`; (4)
  `CancelAuthentication` makes it fail `...Error.Cancelled` and clears the view; (5)
  `Policy(Off)` unregisters. `dev/accept-polkit.sh` on the nested compositor with a debug daemon
  (`sill debug polkit` needs no registration, so nothing reaches the system bus): (1)
  `sill-polkit-<out>` exists hidden; (2) `sill debug polkit` shows it (capture,
  `dev/out/polkit-cosmic.png`) and `sill polkit status` says `showing` the action; (3) typing is
  lane d's (debug injection has no keys); (4) 0 frames while hidden.

**(c) c-switcher: the app switcher** (H5).
- Owns: `crates/sill-services/src/switcher.rs`; `crates/sill-surfaces/src/surfaces/switcher/**`
  (the implemented `machine.rs`, `layer.rs` and `link.rs` may change inside, never in
  signature); new `tests/{switcher_render,switcher_lint}.rs`; `crates/sill/src/switcher.rs`;
  `dist/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions` (the two switcher
  lines); a new `dev/accept-switcher.sh`.
- Frozen: `SwitcherApp`, `AppShown`, `switcher_apps`, `activate`, `quit`, `hide`; the machine's
  public API; `SwitcherPanel { output }`, `switcher_layer`, `PANEL_ID`, `PANEL_RADIUS`,
  `SwitcherLink`, `SwitcherAsk`, `SwitcherAsked`, `AskSerial`, `Chord`, `sw_key`,
  `chord_after_keyup`, `tiles`, `metrics_of`, `key_of`, `index_of`, `output_width`;
  `crate::switcher::{spawn_missing, namespace, ask_of, dispatch, fake, target}`.
- Done: the 5 `c-switcher` tests pass un-ignored; the panel as `panel.rs`'s doc says;
  `system_actions` binds the two switcher actions to `sill switcher next|prev`.
- Verify: the gates; `dev/accept-switcher.sh` on the nested compositor (a debug daemon, three
  `foot` windows the script starts on `wayland-1` with distinct app ids, never the user's): (1)
  `sill-switcher-<out>` exists hidden; (2) `sill debug switcher a b c` shows the panel at once
  (capture, `dev/out/switcher-cosmic.png`), `sill switcher next` moves the ring, `sill switcher
  cancel` hides it and no toplevel changed; (3) `sill switcher next` then `sill switcher commit`
  within 100 ms activates the previous app with no panel frames; (4) `next`, 300 ms, `next`,
  `commit` activates the third app in MRU order; (5) 0 frames while hidden.

**(d) d-real-input: after a, b and c are merged** (H5).
- Owns: a new `dev/accept-m11-real-input.sh` and its entries in `docs/manual-checks.md`. No
  source file: a bug is reported against its lane's files.
- Done and verify: on shell-host's headless KWin (`scripts/dev/kwin-virtual.sh start`,
  `fake-input.sh`, one fake-input client for the whole run: G50), the daemon on KWin's private
  bus: (1) the switcher with real keys: `key Alt_L down`, `sill switcher next`, 300 ms, `key
  Tab`, `key Alt_L up`: the panel showed and the release activated (the key-up path); the same
  with the release 30 ms after the chord records whether the quick tap is seen or waits on G180;
  Escape; Q on a window the script started (it closes); H (it minimises); (2) the polkit prompt
  with real typing: `sill debug polkit`, a wrong secret and Return (it shakes and stays),
  `sill-fake` and Return (it hides), Escape on a second one cancels it; (3) Caps Lock on shows the
  mark on the polkit prompt. The lock screen's real typing is a manual check: KWin has no
  `ext-session-lock-v1` (F551) and fake input cannot reach the nested cosmic-comp (F52). KWin is
  stopped at the end.

Needs a sill login (`docs/manual-checks.md`, lanes a and b add them): a real password on the own
lock screen, lock before sleep and on logind's Lock, a crash while locked (kill the daemon: the
session script restarts it and it re-locks), `pkexec true` answered by sill's own prompt.

### Gaps found in the M11 freeze

- **Q240 (quire), closed during the freeze** The M11 parts (`LockClock`, `LockPrompt`,
  `PolkitPrompt`, `AppSwitcher`, `LockScreen`) landed on quire master `89f0e37` (F550).
- **Q241 (design/22)** Rows for §3.19 (`tests/keys.rs` `AWAITING_ROWS`), and the two existing
  rows' alternatives:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `session.locker` | `String` | `"auto"` | `sill` (sill's own lock screen, M11), `auto` (cosmic-greeter, else swaylock, else hyprlock, whichever is installed; under cosmic-session, COSMIC's own resident locker), `off`, `cosmic-greeter`, `swaylock`, `hyprlock`, or a command line run by `/bin/sh -c` | `20-SURFACES.md#1-9-lock-screen-spec-tier-1`; sill FINDINGS "M7", "M11 freeze" F544 | proposed (2026-09-26) |
  | `session.polkit_agent` | `String` | `"auto"` | `sill` (sill's own agent and prompt, M11), `auto` (polkit-kde, polkit-gnome, lxqt-policykit, polkit-mate, then cosmic-osd last), `off`, one of those names, or a command line run by `/bin/sh -c` | `20-SURFACES.md#1-10-polkit-prompt-spec-tier-1`; sill FINDINGS "M7", "M11 freeze" F546 | proposed (2026-09-26) |
  | `session.lock_clock` | `LockClock::{H24,H12}` | `H24` | how sill's own lock screen writes the time: `14:05`, or `2:05` without AM/PM (macOS's lock screen) | `20-SURFACES.md#1-9-lock-screen-spec-tier-1`; sill FINDINGS "M11 freeze" F553 | proposed (M11 freeze, 2026-09-26) |

  All Advanced, page Accounts. Later, not registered (F553): `session.lock_grace_s` (`Seconds`,
  `0`, `0..=300`: after an idle lock, any key within this opens it without a password; macOS's
  "Require password after sleep or screen saver begins"). design/20 §1.9's and §1.10's milestone
  rows become "own at M11 behind `session.locker = sill` / `session.polkit_agent = sill`;
  borrowed stays the default until the checks in F544 pass".
- **Q242 (design/13)** §13.3.5's quick tap is "released before the show delay", and design/22
  also has `switcher.quick_tap_ms` (100): two numbers for one rule. The frozen machine uses the
  show delay only; design/13 should say what `quick_tap_ms` adds, or the row should go.
- **Q243 (quire)** `LockUser`'s avatar is an initials `AvatarFace`; the person's picture
  (AccountsService's `IconFile`, `~/.face`: `auth::Account.avatar`) has no slot yet (announced as
  coming, with an animated picture).
- **G180 (shell-host)** The keyboard's modifiers when a surface gains focus, and every
  modifier-only change after: `wl_keyboard.enter` carries the held keys and a `modifiers` event
  follows, but a document hears modifiers only on its next key or pointer event. The switcher
  must know at focus whether the chord's modifier is already up (a quick tap that beat the map),
  and the lock prompt whether Caps Lock is on before the first key. Wanted: `use_modifiers() ->
  ReadSignal<Modifiers>` per focused surface (or the modifiers on `HostEvent::KeyboardFocus`).
  Until then a quick tap that races the map leaves the switcher up until a key (Escape, or
  pressing and releasing the modifier) closes it, and the caps-lock mark appears at the first key.

## G190 hot-corner re-arm test flakes under heavy load (2026-09-26)

- `hot_corners_render::leaving_and_the_rearm_lets_it_fire_again` failed once at load average 73
  (three agents building) and passed 5/5 on rerun. The harness advances simulated time, but the
  corner's re-arm window elapsed on the wall clock under that load, so a second dwell fired. Fix:
  drive the corner machine's timers from the harness clock (as `ds::sleep` does under the
  Harness) or assert with a settle loop; until then treat a single failure under load as noise.

## Battery widget adopts quire's round-four ring composition (branch `battery-rings`, 2026-09-26)

- F532 quire's `BatteryLevel` (design/23-WIDGETS.md §4.1, the fourth pass "Flat, bright,
  measured") now draws only the 64 px ring (track, arc, the device's glyph, a bolt at twelve
  while charging); it no longer draws the percentage or the name. `sill-surfaces/widgets/
  battery.rs` composes the rest, copying `ds-gallery`'s "Widget reference" page exactly: a
  `BatteryLayout` enum (`Solo`/`Grid`/`Row`) picked from the host and the row count. Desktop with
  one battery is `Solo` (the ring top left, the percentage alone as the card's hero figure,
  `--fs-widget-hero` 500); Desktop with several is `Grid` (a 2 x 2 of rings, empty places padded
  to four at 0, no numbers); the tile is always `Row` (up to four rings, the percentage under
  each at `--fs-widget-figure`). Device names are no longer printed anywhere in the widget (only
  in the ring's `aria-label`, matched by `ring_label`); `widget_battery.css` is placement and
  tokens only, mirroring the gallery's own `g-wr-*` rules (the row's extra `--s-8` padding on
  top of the frame's own 12 px inset lands the pitch at the measured 20/80 px, M15).
- F533 M25 ("no header on a glanceable widget"): `frame_head` now gives Battery and World Clock
  no title, matching the gallery composing `WidgetFrame` with none for either. Checked World
  Clock's own composition against the gallery's `ClockMedium`/`ClockSmall`: sill only ever draws
  it at `WidgetSize::Medium` (both hosts), already picks Analog on the desktop and Digital in
  the tile correctly (`look_of`), so nothing else changed. One difference kept as is (out of
  this task's "placement only" scope, and not a regression): the gallery's reference draws a
  city's day and its offset as two separate lines; sill's `ClockRow.offset` is one combined
  string ("Today, -7h") rendered as a single `span.sill-widget-city-offset`. Splitting it into
  two lines is a small data-model change (`ClockRow` growing a second field, plus its tests),
  not a CSS placement change, so it is left for a future pass.
- Live: `dev/accept-widgets.sh` re-run on the nested cosmic-comp (wayland-1, never stopped):
  PASS all six steps. `sill debug capture`'s five-tile center shows the Battery tile with no
  header ("No batteries", the private bus has neither UPower nor BlueZ) and the World Clock
  tile with no header either (digital digits, city, one combined day/offset line). A new
  harness test, `sill-surfaces/tests/battery_snapshot.rs`, renders the Battery widget against
  fake services (`ds_native::snapshot`, headless) at Small (alone and several) and Medium, to
  `target/battery-snapshots/*.png`, since the private bus never has a real battery to capture
  live; all three match the gallery reference's own composition.
- F621 (superseded: switcher.quick_tap_ms was retired instead, Q242; the key is gone and AWAITING_ROWS is empty) Merging master (the M11 freeze) turned up one gate failure unrelated to this branch:
  `sill-settings::keys::every_spec_has_a_row_in_the_catalogue` failed on
  `switcher.quick_tap_ms`. F553 ("`switcher.*` already has its rows … nothing is added there")
  was stale by the time this merge landed: the sibling quire checkout's design/22-SETTINGS.md
  §3.11 has `show_delay_ms`, `icon_size_px`, `cell_size_px`, `cell_gap_px` and
  `overflow_min_icon_px`, but no `quick_tap_ms` row. Fixed the only way this test allows,
  the same as `session.lock_clock`: added `switcher.quick_tap_ms` to `keys.rs`'s
  `AWAITING_ROWS`, no switcher logic touched. quire's owner adds the row; then the entry can go.

## M11 lane a: sill's own lock screen (2026-09-26)

- F560 PAM runs in a `sill auth-check` child per attempt (nonstick), the password on its stdin
  as NUL-terminated fields, never argv or env, killed after 15 s; the daemon never holds a PAM
  handle. Every copy is zeroed (the request bytes, the child's fixed read buffer, the Secret, the
  PAM conversation's answers). A made-up verdict (`sill debug auth`) exists only in debug builds.
- F561 A latch (`crates/sill/src/lock/follow.rs`) keeps the lock released until the locker
  reports `Unlocked` or a new `Locking`, so an auth reset reaching the follower first cannot
  re-lock. The crash marker is `$XDG_RUNTIME_DIR/sill/locked` (Own mode only).
- F562 The prompt starts on the main display, then follows whichever lock surface gains keyboard
  focus or a click; whether cosmic-comp focuses the pointer's output first is a manual check.
- F563 The picture: AccountsService's icon, then `$HOME/.face` (the frozen order), else the
  initial face; never a persona (the user chose animated emoji, a later quire break).
- F564 The unlock fade runs at `--t-move` (250 ms), shorter than the 454 ms accepted beat, so the
  beat is mostly faded before the unlock; a delayed fade is a follow-up.
- Q271 (was numbered Q270 in lane a's notes, which is lane c's range) Proposed rows (not registered; the user decides whether lockout is configurable):
  `session.lockout_free_tries` u32 5, `session.lockout_first_wait_s` 60,
  `session.lockout_max_wait_s` 900, Advanced, Accounts page. Today these are constants.
- G191 closed: `debug stats` lists `sill-lock/<output>` while a lock is up.
- Acceptance `dev/accept-lock.sh` on the nested compositor: 8/8 (lock 126 ms, a second locker
  refused, wrong shakes and clears, lockout after the sixth wrong, 0 commits idle, unlock 742 ms,
  crash re-lock 2 ms after restart, a non-debug build refuses a made-up verdict).

## M11 lane c: the app switcher (2026-09-26)

- F600 Apps are grouped by desktop entry (else the raw app id) in MRU order; activate, quit and
  hide become window commands through cosmic_wl. A second chord inside the show delay is a quick
  tap (switch to the previous app with no panel); an opening that reaches the show delay without
  ever reading the chord held is also treated as a quick tap.
- F601 Only a modifier's own key-up releases: a virtual keyboard's modifiers reach only the focused
  client, and a non-modifier key-up from it carries none.
- F602 dist/cosmic system_actions: `WindowSwitcher: "sill switcher next"`,
  `WindowSwitcherPrevious: "sill switcher prev"`.
- Acceptance `dev/accept-switcher.sh` on the nested compositor (its own three windows, wtype on
  the nested virtual keyboard): 7/7; the no-commit quick tap is NOT PROVEN there because the
  nested seat reports Alt held from the host session; it needs a real login (manual check).
- G210 (shell-host) Unmapping a surface rebuilds its wl_surface, so the compositor's keyboard
  leave names the destroyed one: `use_modifiers` is never reset and keyboard focus goes stale;
  the switcher takes the value at mount as each opening's baseline. Wanted: withdraw the keyboard
  on unmap, as G101 did for the pointer.
- Left open: keyboard focus arriving after the show delay (50–90 ms here) would read a held
  chord as a tap.

## After the identity rewrite (2026-09-26)

- History rewritten to `PoHsuan Lai <pohsuanlai0208@gmail.com>` (user-approved; same trees);
  sill origin/master dab40b8 -> 57a7343, shell-host a32b936 -> e6638c5. Gates pass on both.
- G210 closed by shell-host F67 (the keyboard is withdrawn on unmap and close). The switcher keeps
  its mount-time baseline as a harmless guard (`switcher/watch.rs`).

## M9 freeze (2026-09-26)

Launcher v2 (design/20 §1.3 "M9 (v2 providers)", design/13 §13.3.9, macOS Spotlight as the
reference): four new providers (files, clipboard history, emoji, the web fallback), currency in
the calculator, Spotlight's sections with Top Hit and "Show More", and a Quick Look preview pane,
frozen on `m9-freeze` so four agents can fill them in parallel and a fifth can prove them under
real input. Everything compiles; every pure function a lane fills is `todo!()` with its table
test `#[ignore = "M9 fill: <lane>"]` (33: 8 a-files, 6 b-clipboard, 6 c-emoji-web-units, 13
d-preview), every new provider's `query` answers nothing, the clipboard service's live loop never
connects, `LaunchCommand::Open` logs that it is not filled, and both new components draw
nothing, so the suite stays green and `sill run` starts. Findings are numbered from
F640/Q290/G230; the lanes take F660/Q310/G240 (a), F680/Q320/G250 (b), F700/Q330/G260 (c),
F720/Q340/G270 (d) and F740/Q350/G280 (e).

Arc heuristics (quire design/09 §2): the work serves **H5, fewer icons, more shortcuts** (every
new row action carries its chord, shown beside it in the actions menu and the preview pane: ⌘R
reveal, ⌘C copy, ⌘B search the web, ⌘Y Quick Look, ⌘⌫ forget; Space and → reach the preview
without the pointer; F642, F654) and **H4, integrate related tools, don't silo them** (what you
copied, an emoji, a document, a conversion and a web search are one launcher keystroke away from
any surface, without switching app or Space; F647, F649, F650).

### What is frozen

- **F640 Dependencies.** Three, sill-only, outside the pinned block, all under the allowed
  licences (`cargo deny check licenses` ok): `emojis` 0.9 ((MIT OR Apache-2.0) AND Unicode-3.0:
  every fully-qualified emoji, its CLDR short name, group, skin-tone variants and GitHub
  shortcodes), `mime_guess` 2.0 (MIT: a file's type from its name when the index gave none; pulls
  `mime` and `unicase`, MIT/Apache), `percent-encoding` 2.3 (MIT OR Apache-2.0, already in the
  tree: the web template's `{query}`). CLDR keywords are in no maintained crate (the `emoji`
  crate's 47 MB of annotations dates from 2021 and declares its Unicode data MIT), so
  `crates/sill-services/data/emoji-keywords.tsv` bundles CLDR 48.2's English annotations (1966
  lines, 88 KB, the Unicode-3.0 header kept), generated by `scripts/gen-emoji-keywords.sh` from
  `/usr/share/unicode/cldr/common/annotations/en.xml`. **No new crate for the clipboard**:
  `ext_data_control_v1` is in the pinned `wayland-protocols` 0.32 (`staging`) and
  `zwlr_data_control_v1` in `wayland-protocols-wlr`, which sctk re-exports
  (`cosmic_client_toolkit::sctk::reexports::protocols_wlr`); `clipboard/wl.rs` names both managers
  so the freeze proves they build. Tracker is zbus (pinned); Baloo, `fd` and `plocate` are
  processes. The lockfile also moved with quire's path dependencies (pdfrum from git to crates.io
  0.4).
- **F641 What a row is about, and clipboard privacy in the vocabulary.** `Item` gains `subject:
  Subject::{None, App(AppId), File(FileFacts{path, mime, modified}), Clip(ClipFacts{key, kind,
  copied}), Emoji(EmojiFacts{glyph, name, tone}), Web(WebFacts{engine, url})}` (serde default
  `None`: the M4 rows are unchanged, apps carry `App`). A clipboard entry travels as a `ClipKey`
  plus an `Excerpt`, whose `Debug` prints its length only; the service's `ClipText` and
  `ClipImage` redact themselves too, and nothing of the clipboard is `Serialize`.
  `frecency::counting` and `ItemKey::counting` make a clipboard row's use **never written to
  `frecency.json`** (the runner records only `Counted` keys); the web row's id is fixed
  (`web:search`), so what was searched is never written either.
- **F642 Chords on actions (H5).** `Action` gains `keys: ActionKeys::{None, Chord(Chord)}`;
  `Chord { mods, key }` is written in Mac terms (quire design/27 §6.2 (e): Command, which the
  platform layer resolves; Toshy maps it to Ctrl), kept in ⌃⌥⇧⌘ order; `chords::{reveal ⌘R,
  copy ⌘C, search_web ⌘B, quick_look ⌘Y, forget ⌘⌫}` (Spotlight's where it has one);
  `action_for_chord` finds the action a chord runs straight from the results. The M4 providers'
  actions carry `None` for now.
- **F643 Activations.** `Activation::Open { target: OpenTarget::{File(path), Reveal(path),
  Url(String)} }`, `SetClipboard { entry }`, `ForgetClipboard { entry }`, wire forms pinned
  (`{"kind":"open","v":{"target":{"kind":"reveal","v":"/home/p/a.pdf"}}}`,
  `{"kind":"set_clipboard","v":{"entry":9}}`). The runner leaves all three to the surface (it
  holds the activation token and the clipboard control): `LauncherEffect::{Open, SetClipboard,
  ForgetClipboard}` are implemented in `run.rs` (`Open` takes a token from the panel before it
  hides and sends `LaunchCommand::Open(OpenRequest{target, token})`; the clipboard ones go to
  `ClipboardCommand::{Offer, Forget}`); forgetting keeps the launcher open. `launch/open.rs` (lane
  a) does the opening: `xdg-open` in a transient scope with `XDG_ACTIVATION_TOKEN`, a reveal
  through `org.freedesktop.FileManager1.ShowItems` when the name has an owner, else `xdg-open` on
  the parent.
- **F644 The preview.** `sill_launcher::Preview::{None, Image(path), Text{excerpt},
  TextFile(path), Pdf{path, page: PageIndex}, File(FileFacts), App{app, name, detail},
  Clipboard(ClipFacts), Emoji(EmojiFacts), Web(WebFacts)}` and the pure `preview_for(&Item)`
  (rules in its doc, pinned by `preview_tests.rs`), `excerpt_of(bytes, ExcerptLimit)` (binary is
  empty, UTF-8 read lossily, cut on a character). `sill_services::preview::{read_excerpt (at most
  16 KiB read), clip_image}` load what a preview names. A PDF is drawn by quire (F657), never
  rasterised or cached by sill.
- **F645 Sections, weights and caps** (`sill_launcher::rank::sections`). `Section::{TopHit,
  Actions, Recent, Applications, Documents, Windows, Calculator, Settings, Clipboard, Emoji,
  System, Web}`; `SectionPlan::default_plan()` gives each provider a `Weight` (thousandths of its
  score in the Top Hit contest) and a `Cap` (rows before "Show More"): apps 1000/6, files 850/5,
  windows 900/3, calculator 1000/1, settings 700/3, clipboard 600/3, emoji 500/8 (a grid row),
  system 800/3, web 0/1 (never Top Hit, always last); a `ceiling` of 20 rows published per
  section, `recent` 5. `place(merged, plan, QueryShape, &Frecency, now) -> Placement { items,
  spans }` and `shown(&Placement, plan, &Expanded) -> Vec<ShownSection { section, rows, more:
  More::{Nothing, Hidden(n), Expanded} }>` are lane d's; `weighted` is implemented. `Results`
  gains `sections: Vec<SectionSpan>` (serde default): empty while the runner still publishes the
  M4 grouping (F114, whose `group_cap` now has arms for the new kinds), which lane d replaces
  with `place`. The numbers are proposed and have no settings keys (a key is a design/22 row
  first). `ProviderKind` gains `Files`, `Clipboard`, `Emoji`, `Web` after the M4 five, so a tie
  still goes to the older kinds and Shift+Tab from All now lands on Web.
- **F646 The launcher policy.** `sill_services::LauncherPolicy { files: FilesChoice::{Auto,
  Tracker, Baloo, Walk, Off}, files_budget, web: WebPolicy::{Off, On{template}}, emoji_tone,
  currency: CurrencyPolicy::{Off, Ecb} }`, all off by default, reaches the providers through
  `SearchCommand::Policy` (the runner keeps it in a `watch` the registry's providers hold). The
  daemon's `crates/sill/src/launcher_policy.rs` (implemented and tested) sends it, and the
  clipboard's, at start and on every settings change, so **`Services::spawn` in a test never
  reads a file index, the clipboard or the network**. The registry (`results/registry.rs`) now
  builds all nine providers, so no lane edits it.
- **F647 The clipboard history service** (`sill_services::clipboard`, `Services.{clipboard,
  clipboard_control}`, hook `use_clipboard`). Its own thread and Wayland connection (as
  `cosmic_wl`), binding `ext_data_control_manager_v1` else `zwlr_data_control_manager_v1`,
  reading each new selection whole through a pipe, never the primary selection. **In memory
  only**: nothing on disk, nothing logged; `HistoryMode::Off` holds no connection and no entries,
  and turning history off forgets everything. **Password managers' copies are never read**:
  `offer_verdict` skips an offer carrying any of `SECRET_HINTS` (`x-kde-passwordManagerHint`, its
  `application/` form, `org.nspasteboard.{Concealed,Transient,AutoGenerated}Type`) before any of it
  is received, on the hint's presence alone (stricter than klipper, which reads the hint's
  value). Pure: `reduce_clipboard(ClipState, ClipEvent) -> (ClipState, Vec<ClipEffect>)` (newest
  first, capped, a re-copy moves to the top under its old key so offering an entry back does not
  duplicate it), `offer_verdict` (text before image; PNG preferred). Passive until the daemon's
  first `ClipboardCommand::Policy`; `Setup { display: ClipDisplay::{Environment, Socket(path)} }`
  lets a live test connect to the nested compositor only. Rows: `ClipboardProvider` (Enter
  `SetClipboard`, "Forget" `ForgetClipboard`, both implemented); `clip_items` is lane b's.
- **F648 Files** (`providers/files/`). The seam `trait FileIndex { kind(); search(&FileAsk,
  &Query) -> FileAnswer::{Hits, Partial, Unavailable(IndexFault)} }` with `TrackerIndex`
  (`org.freedesktop.Tracker3.Miner.Files` SPARQL over zbus: `sparql_for`, `escape_literal`),
  `BalooIndex` (`baloosearch6`: Baloo's D-Bus has no query API; `baloo_args`, `parse_paths`),
  `WalkIndex { walker: Fd | Plocate }` (`fd_args`, `plocate_args`, killed at the budget) and a
  test `FakeIndex`; `choice::{pick, detect}` (Auto: Tracker, Baloo, fd, plocate; a named backend
  that is missing is not substituted); `hits::{file_items, tilde}` rank every backend alike (name
  match, frecency, a recency boost). `FilesProvider { policy, frecency, source:
  IndexSource::{Detect, Given}, home }`; its `activate` is implemented (Enter opens, ⌘R reveals,
  ⌘C copies the path). Every argument vector passes the typed text as one argument after `--`;
  SPARQL literals are escaped (tables pin both). Tests use `FakeIndex` and temp directories;
  nothing reads the person's index or home.
- **F649 Emoji** (`providers/emoji/`). `table::{KEYWORDS_TSV, parse_keywords, emoji_table}` (the
  `emojis` set joined with the CLDR keywords), `search::{emoji_items, toned}`: names first, then
  keywords, then shortcodes (`:tada`), recent first when asked alone with nothing typed (F651),
  the skin tone applied where it can be, the frecency key always the default-tone glyph. Enter
  copies (the launcher cannot type into another app). The data model says nothing about painting:
  a cell carries text, and the system's COLRv1 face paints it (F657).
- **F650 The web fallback and currency.** `WebProvider` (one row, `web:search`, last, only while
  `launcher.web_search` is on): `search_url` refuses a template without `{query}` or not
  `https`/`http` (a setting must not open `file:` or `javascript:`). Suggestions from the engine
  are later and have no key. Currency: `calculator/rates.rs` (`parse_ecb`, `relative_to_euro`,
  `SharedRates`); with `launcher.currency = "ecb"` lane c fetches the ECB's daily file with `curl`
  at most once a day into `$XDG_CACHE_HOME/sill/ecb-rates.xml`, off the keystroke path, and hands
  fend an `ExchangeRateFnV2`; `off` (the default) fetches nothing. Units already work (fend).
  `CalculatorProvider` is now a struct (`policy`, `rates`; its `Default` is the M4 one).
- **F651 A query asked of one provider.** `Query` gains `scope: Scope::{All, Only(kind)}`
  (`Query::begin_only`) and the runner `SearchCommand::Filtered { text, provider }`
  (implemented: only that provider runs). The Tab filter sends it, so a filtered provider is no
  longer capped by the others' grouping (F114's limit), and a provider asked alone may list what
  it has for nothing typed (the emoji grid, the clipboard's newest) without flooding the launcher.
- **F652 Settings** (`sill-settings/src/{launcher.rs, launcher_kinds.rs}`, all in
  `AWAITING_ROWS`, rows in Q301): `launcher.files_backend` (`auto|tracker|baloo|fd|off`, `auto`),
  `files_budget_ms` (150), `clipboard_history` (`memory|off`, `memory`), `clipboard_cap` (50),
  `clipboard_image_max_mb` (8), `emoji_skin_tone` (`default`), `web_search` (`on|off`, `on`),
  `web_engine_url` (`https://duckduckgo.com/?q={query}`), `currency` (`off|ecb`, `off`). All
  Advanced, because design/22 §5 has no launcher page; Q303 asks for one with
  `clipboard_history`, `web_search` and `emoji_skin_tone` Basic, since the history is a privacy
  choice a person should find without the file.
- **F653 IPC and CLI** (`sill-ipc/src/launcher.rs`, `crates/sill/src/{cli_launcher.rs,
  clipboard.rs, launcher_debug.rs}`, wire lines pinned in `sill-ipc/tests/serde.rs`):
  `Request::Clipboard(ClipboardRequest::{Status, Clear})` (`sill clipboard status|clear`,
  implemented: `Response::Clipboard(ClipboardReport { mode, protocol, entries })`, counts only);
  `DebugRequest::LauncherKey(FakeLauncherKey)` (`sill debug launcher-key
  up|down|enter|escape|tab|shift-tab|actions|space|quick-look|right|left|reveal`) and
  `DebugRequest::LauncherRows` (`sill debug launcher-rows`, `Response::LauncherRows { query,
  sections: [{title, rows, hidden}], selected, preview }`, clipboard titles replaced by
  `"<clipboard>"`), refused by a daemon built without `debug`, answered "not filled yet" until
  lane d.
- **F654 The preview pane and its keys** (`surfaces/launcher/preview/`). `PreviewPane { preview,
  actions, focused }` (draws nothing until quire's part, Q292), `PREVIEW_WIDTH_PX` 360 beside the
  540 results. `keys::step_preview(PreviewState { pane, focus, edit }, PreviewKey, Previewable,
  controls)`: **Space toggles the pane only while browsing** (the selection last moved by Up or
  Down, nothing typed since; while typing it is a space in the query, because the field has the
  keyboard, unlike Finder's Quick Look); **⌘Y** toggles it always; **→ at the caret's end** shows
  it and a second → enters it, **⌘→** always does; in the pane **Tab and Shift+Tab walk its
  controls** (quire design/27 §6.4 "every control in the Tab order"; in the results Tab stays
  the provider filter, design/06 §20.2); **← or Escape** give the keyboard back; Escape with the
  pane shown hides it; only then does Escape close the launcher. design/13 has no row for any of
  this (the brief's "→/⌘→ as design/13 says" has no text behind it in design/13 today): Q302
  proposes the rows.
- **F655 Motion is a hook point, not sill CSS.** `preview/moment.rs`: `PaneMoment::{Still,
  Appear, Swap, Pending, Leave}`, the pure `pane_moment(before, after)`, and `use_pane_moment`,
  `use_row_moment` (no-ops) are where quire's D0 small-details driver (`use_detail`, moments
  `Appear`, `Pending`, `Preview`) is called, and nowhere else (Q297).
- **F656 Rows and the grid.** `rows::{RowKind, row_kind, RowDetail, row_detail}`: a file row
  (name, `~/…`, modified as the mail list words a time), a clipboard row (excerpt or image, age),
  an emoji cell, a web row; until quire's row variants land (Q290) they go through the palette's
  `MenuEntry::Item` (title, detail, `Trail::Note`). `sections_view::{HeaderAction::{None,
  ShowMore, ShowLess}, palette_sections}`. `emoji_grid::{EmojiGrid { cells, selected, onpick },
  GRID_COLUMNS 8, grid_step}`: the grid and every cell carry quire's `ds-emoji` class (the
  `--font-emoji` token; sill never writes `font-family`, which quire's lint refuses). **Open
  risk: a scrolling grid of a few hundred colour glyphs at 60 Hz on vello_hybrid is untimed**;
  lane c times it before the grid relies on it. The M4 glyph tables name `File`, `Copy`,
  `Sparkles` and `Search` for the new kinds until quire has their glyphs (Q296).
- **F657 quire's m9-parts, frozen against their announced shape.** PDF:
  `ds_native::PdfFileThumb { path: PathBuf, size: Size, label: Option<String> }` behind
  `ds-native`'s `pdf-thumb` feature (off by default; lane d enables it on `sill-surfaces`' normal
  `ds-native` dependency when it lands), over the pure `ds::PdfThumb { page: PdfPage::{Loading,
  Ready{image, sheet}, Empty, Failed(PdfTrouble::{Unreadable, Locked})}, size, label }`; quire
  keys its cache on path, mtime, size and scale, so sill passes the path and keeps no cache; its
  classes (`ds-pdf-thumb`, `-sheet`, `-page`, `-plate`, `data-state`, `data-trouble`) are never
  styled by sill. Emoji face: quire adds `--font-emoji` and `.ds-emoji` with m9-parts, and its
  System and Editorial stacks gain the emoji face, so emoji in ordinary text (a row's title)
  already paint in colour; CBDT is out (Q295, answered). The user-picture picker uses
  `AnimatedEmoji`'s still frames; the launcher does not use `AnimatedEmoji`.

### The M9 fill-in: four agents, disjoint files, then real input

Every agent reads CONVENTIONS §0 and §1 (fill bodies, add private helpers and tests; never change
a frozen signature, public field or variant; stop and report instead), ORCHESTRATION "Coherence
rules", design/20 §1.3, design/13 §13.3.9, quire design/09 §2 (H4, H5), design/27 §6.2 and §6.4,
quire CONSUMING "CommandPalette" (the launcher gaps and the palette follow-ups) and this section.
Everyone runs, before reporting:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features sill/debug
./scripts/check-boundary.sh
cargo deny check licenses
```

**Privacy, for every lane.** No test reads the person's clipboard, file index or home directory
beyond detecting that a backend exists: use `FakeIndex`, temp directories, a scratch `HOME` for a
daemon, and the nested compositor with the test's own data. Clipboard content is never logged,
printed in a test's failure message, or written to disk. Nothing types into the person's session
(`wayland-0`): scripts ping `wayland-1` first and never stop or restart it (`wtype` only with
`WAYLAND_DISPLAY` set to the nested socket's absolute path); a daemon under test gets its own
`XDG_RUNTIME_DIR` and a scratch `XDG_CONFIG_HOME`.

Files not listed under an agent are read-only to all: in particular sill-launcher's `lib.rs`,
`activation.rs`, `ids.rs`, `item.rs`, `subject.rs`, `chord.rs`, `query.rs`, `frecency.rs`,
`rank.rs`; sill-services' `lib.rs`, `services.rs`, `results/{mod,registry,policy,effect,model}.rs`,
`launch/{mod,run}.rs`; `sill-surfaces/src/hooks/**` and `tests/support/fake.rs`;
`sill-settings/**`; `sill-ipc/**`; `crates/sill/src/{daemon.rs, cli.rs, cli_launcher.rs,
launcher_policy.rs, clipboard.rs}`; the root `Cargo.toml` (a lane that needs a dependency stops
and reports).

**(a) a-files: Documents.**
- Owns: `crates/sill-services/src/providers/files/**`, `crates/sill-services/src/launch/open.rs`,
  new `crates/sill-services/tests/files_*.rs`, new `crates/sill-services/examples/*.rs` and their
  `[[example]]` entries, a new `dev/accept-launcher-files.sh`.
- Done: the `a-files` tests pass un-ignored; `FilesProvider::query` as its module doc says (no
  index asked for nothing typed or `Off`; one `FileAsk` under `home` within the policy's budget;
  a `Partial` answer still pushed; `file_items` ranks); `detect` cached once per provider;
  `TrackerIndex` over zbus (`Endpoint.Query` with a pipe, `Handle::block_on` as F107 does, the
  budget as a timeout), `BalooIndex` and `WalkIndex` as processes killed at the budget; `open`
  runs `open_argv` in a scope with the token, a reveal through `FileManager1.ShowItems` first.
- Verify: the gates; `dev/accept-launcher-files.sh` on the nested compositor, the daemon with a
  scratch `HOME` holding `Documents/m9-report.pdf` and `notes/m9-notes.md` and
  `launcher.files_backend = "fd"`: (1) `sill launcher open --query m9-rep`, then `sill debug
  launcher-rows` lists `m9-report.pdf` under Documents (once lane d answers rows; until then the
  lane's live test drives the provider directly); (2) Enter opens it through a scratch
  `mimeapps.list` naming a stub `.desktop` that records its argument; (3) ⌘R reveals its folder
  (a stub file manager records it); (4) `files_backend = "off"` and `sill reload`: no Documents
  section; (5) a live test (`#[ignore = "needs tracker"]`) only checks `detect().tracker` here and
  reports it.

**(b) b-clipboard: the history.**
- Owns: `crates/sill-services/src/clipboard/**` (the model types frozen; private files may be
  added), `crates/sill-services/src/providers/clipboard.rs`, new
  `crates/sill-services/tests/clipboard_live.rs` (and a support module of its own), new examples
  (a source offering text plus `x-kde-passwordManagerHint`), a new `dev/accept-clipboard.sh`.
- Done: the `b-clipboard` tests pass; `live::run` connects only under `HistoryMode::Memory`, on
  its own thread, binds ext else wlr on the first seat, follows `selection` (never
  `primary_selection`), asks `offer_verdict` before receiving anything, reads the chosen type
  through a pipe with a size limit (images up to the policy's cap, their size read with `image`),
  reduces, publishes, and on `OfferSelection` creates a data-control source offering the entry's
  type; `Disconnect` drops the connection; nothing is logged but counts and fault kinds;
  `clip_items` and `query` as the provider's doc says.
- Verify: the gates; `cargo test -p sill-services --test clipboard_live -- --ignored` against the
  nested compositor by socket path; `dev/accept-clipboard.sh` on the nested compositor (daemon
  with a scratch config): (1) `sill clipboard status` says `memory`, `ext` (cosmic-comp offers
  it), 0 entries; (2) `wl-copy` of a made-up string on the nested socket: 1 entry; (3) the example
  source offering text with `x-kde-passwordManagerHint`: still 1; (4) `sill launcher open --query
  <a word of it>` shows one Clipboard row, Enter, then `wl-paste` on the nested socket answers the
  string; (5) `launcher.clipboard_history = "off"`, `sill reload`: `off`, `not_connected`, 0
  entries; (6) back on, two copies, `sill clipboard clear`: 0.

**(c) c-emoji-web-units: emoji, the web fallback, currency.**
- Owns: `crates/sill-services/src/providers/emoji/**`, `crates/sill-services/src/providers/web.rs`,
  `crates/sill-services/src/providers/calculator/**`, `crates/sill-services/data/**`,
  `scripts/gen-emoji-keywords.sh`, a new `crates/sill-surfaces/examples/emoji_paint_bench.rs`.
- Done: the `c-emoji-web-units` tests pass; the emoji provider answers typed text of two or more
  characters, `:` shortcodes, and the recent-first set when asked alone (`Scope::Only`); the web
  provider's one row for typed text while on; currency: `detect.rs` reads ISO codes and currency
  words as units only while `currency = "ecb"`, the daily fetch with `curl` into the cache (never
  while `off`, never on a keystroke), fend's `ExchangeRateFnV2` over `SharedRates`; **the emoji
  paint timing**: a bench drawing 300 `ds-emoji` cells in a `ds_native::Harness`, scrolled at 60
  Hz on vello_hybrid (frame p50, p95, max), reported before lane d's grid relies on it.
- Verify: the gates; the bench's numbers; on the nested compositor (lane d's script covers the
  rows once it can list them): `sill launcher open --query "thumbs"` lists 👍 under Emoji,
  `--query "rust wayland"` ends with "Search the web for “rust wayland”", `--query "5 km to mi"`
  answers `3.10686 mi`, and with `currency = "ecb"` and a pre-seeded cache, `--query "10 usd to
  eur"` converts without a network call (the cache's mtime is today).

**(d) d-preview: sections, the preview pane, the rows, the debug keys.**
- Owns: `crates/sill-launcher/src/{preview.rs, preview_tests.rs, rank/sections.rs,
  rank/sections_tests.rs}` (bodies only), `crates/sill-services/src/results/{arrange.rs, run.rs}`,
  `crates/sill-services/src/preview.rs`, `crates/sill-surfaces/src/surfaces/launcher/**`,
  `crates/sill-surfaces/src/style/launcher.css`, `crates/sill-surfaces/tests/launcher_*.rs`,
  `tests/launcher_support/**`, `tests/launcher_harness/**`, `examples/launcher_demo.rs`,
  `crates/sill-surfaces/Cargo.toml` (the `ds-native` `pdf-thumb` feature only, once quire's
  m9-parts land), `crates/sill/src/launcher_debug.rs`, a new `dev/accept-launcher-v2.sh`.
- Done: the `d-preview` tests pass; the runner publishes `place` with its spans (typed text,
  nothing typed, and a filtered query placing its provider's rows alone) and the panel draws
  `shown` sections with "Show More"/"Show Less"; Tab sends `SearchCommand::Filtered`; the preview
  column follows `preview_for(selected)` through `step_preview`, loads text off the UI thread
  (`read_excerpt`), shows a clipboard image from `use_clipboard` (`ImageSource::png`) and a PDF
  through `PdfFileThumb`; chords run their actions from the results (`action_for_chord`); a
  forgotten clipboard entry re-queries; `launcher_debug::{key, rows}` through the `LauncherLink`;
  every part quire's, stand-ins through `MenuEntry` until Q290-Q294 land, moments only through
  `use_pane_moment`.
- Verify: the gates; `dev/accept-launcher-v2.sh` on the nested compositor (daemon `--features
  debug`, a scratch `HOME` with a PDF, a PNG and a Markdown file, `files_backend = "fd"`): (1)
  open latency unchanged against master (`launcher.open_to_frame_ms`, 10 warm openings); (2)
  `--query m9` lists Top Hit, then Documents (`launcher-rows`); (3) `launcher-key down`, `space`:
  the preview is `shown` `pdf`, a capture shows the page (`dev/out/launcher-v2-preview-pdf.png`);
  (4) `down` to the PNG: `image`; `space` again hides it; (5) `quick-look` while typing shows it;
  `right` twice: the pane has the keyboard; `escape` gives it back, `escape` hides the pane,
  `escape` closes the launcher; (6) a section over its cap reports `hidden > 0`, and Show More
  reveals them; (7) `tab` to Emoji with nothing typed shows the grid
  (`dev/out/launcher-v2-emoji.png`); (8) 5 s idle after closing: 0 frames.

**(e) e-real-input: after a, b, c and d are merged.**
- Owns: a new `dev/accept-launcher-v2-real-input.sh` and its entries in `docs/manual-checks.md`.
  No source file: a bug found is reported against its lane's files.
- Done and verify: on shell-host's headless KWin (`scripts/dev/kwin-virtual.sh start`,
  `fake-input.sh`, one fake-input client for the whole run: G50; KWin stopped at the end, the
  daemon given the bus only: G52), a scratch `HOME` and config: (1) real typing of a file's name
  lists it, real Down then Space opens the preview, real Space closes it, real ⌘Y (Ctrl+Y) opens
  it while typing; (2) real → twice enters the pane, real Tab walks its controls (the focus ring
  on each), real Escape three times closes one level each; (3) real ⌘R on a file row reveals it
  (a stub file manager records it); (4) real Tab to Emoji, real arrows move in the grid, real
  Enter copies (`wl-paste` on KWin's socket); (5) the clipboard: if KWin offers a data-control
  manager, a copy made on KWin's socket shows as a row and real Enter restores it; if not, status
  says `unsupported` and the step is reported (G230).

### Gaps found in the M9 freeze

- **Q290 (quire) Row shapes for files and clipboard entries in the palette**, as `ListRow`
  variants or palette row slots, not new bare markup: `MenuEntry::Item` gains `shape: RowShape`
  (default `Plain`), `RowShape::File { thumb: Option<ImageSource>, location: String, modified:
  String }` (a 34 px tile or a thumbnail, the name as title, `~/Documents` under it, the time as
  trailing data 10) and `RowShape::Clip { body: ClipBody::{Text { excerpt: String, lines: u8 },
  Image { src: ImageSource, size: ImageSize }}, age: String }` (two lines of mono excerpt or a
  thumbnail, the age trailing). A contact row is not M9's and is not asked for.
- **Q291 (quire) `EmojiGrid<T>`**: `cells: Vec<EmojiCell<T> { value: T, glyph: String, name:
  String }>`, `columns: u8`, `cell: Px` (56), `selected: Option<usize>`, `on_select:
  EventHandler<usize>`, `onpick: EventHandler<T>`; 2D arrow keys clamped at the edges, the name
  as each cell's label and tooltip, `ds-emoji` on the grid; and a way to put it in the palette as
  one group (`PaletteGroup::Grid`) so the palette's Up/Down enter and leave it.
- **Q292 (quire) A preview pane**, the future Quick Look's shared part: `PreviewPane { content:
  PaneContent::{Image { src, size }, Text { excerpt, mono: Mono }, Pdf { path } (through
  `PdfFileThumb`), App { icon: IconSource, name, detail }, Facts { icon, title, rows:
  Vec<(String, String)> }, Emoji { glyph, name }, Web { host, url }}, actions: Vec<PaneAction {
  label, shortcut: Shortcut }>, focused: Option<usize>, onaction: EventHandler<usize> }`, the
  actions as buttons in the Tab order with their `Kbd`; and `CommandPalette { aside:
  Option<Element>, aside_width: Px }` so the card widens by the pane and its blur region follows.
- **Q293 (quire) `PdfFileThumb`**: answered by m9-parts (F657).
- **Q294 (quire) "Show More" in the palette**: `CommandPalette.groups` as `Vec<PaletteGroup<T> {
  title, entries, action: Option<(String, EventHandler<()>)> }>`, drawn with `SectionHeader`'s
  trailing action, and reachable from the keyboard (proposed: Down past a capped section's last
  row stops on its header action and Enter runs it), since Tab is the provider filter.
- **Q295 (quire) The emoji face**: answered, `--font-emoji` and `.ds-emoji` in m9-parts (F657).
- **Q296 (quire) Glyphs** for the new providers: `Clipboard` (a clipboard board), `Smile`
  (emoji), `Globe` (the web); the launcher uses `Copy`, `Sparkles` and `Search` until then.
- **Q297 (quire) D0 `use_detail`** for the row appear stagger (`Appear`, cap 12, first show only,
  design/20 §1.3), the pane's `Appear`/`Preview`/`Leave` and a slow load's `Pending`; sill calls
  it from `preview/moment.rs` only.
- **Q298 (quire) `StandardAction`** (design/27 §6.2 rule (a)) with the launcher's keys as data:
  Quick Look ⌘Y, Reveal in Files ⌘R, Copy ⌘C, and a way from sill's `Chord` to
  `Shortcut::standard`, so every trail renders in ⌃⌥⇧⌘ order from one table.
- **Q299 (quire) The palette's field, Space and the caret**: the palette's field inserts Space
  before `onkey` hears it; the preview machine needs Space (and → at the caret's end) handed to
  `onkey` first, so the panel can take it while browsing (`prevent_default`), and the caret's
  position (`Caret::{AtEnd, Inside}`), which the palette does not report.
- **Q301 (design/22)** The rows `AWAITING_ROWS` exempts (F652), for §3.6:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `launcher.files_backend` | `FilesBackend::{Auto,Tracker,Baloo,Fd,Off}` | `Auto` | `auto` asks Tracker 3, else Baloo, else `fd`/`plocate` under the home folder; a named backend that is missing searches nothing | `20-SURFACES.md#1-3-launcher-spec-tier-1`; sill FINDINGS F648 | proposed (2026-09-26) |
  | `launcher.files_budget_ms` | `Ms` | `150` | `20..=2000`; one file search's limit, after which what was found shows | sill FINDINGS F648 | proposed (2026-09-26) |
  | `launcher.clipboard_history` | `ClipboardHistory::{Memory,Off}` | `Memory` | kept in memory only and forgotten when sill stops; password-manager copies never kept; `Off` watches nothing | `09-ARC-HEURISTICS.md#2-the-seven-heuristics` H4; sill FINDINGS F647 | proposed (2026-09-26) |
  | `launcher.clipboard_cap` | `Count` | `50` | `1..=500`; the oldest goes first | sill FINDINGS F647 | proposed (2026-09-26) |
  | `launcher.clipboard_image_max_mb` | `Count` | `8` | `0..=64`; larger images are not kept, `0` keeps none | sill FINDINGS F647 | proposed (2026-09-26) |
  | `launcher.emoji_skin_tone` | `EmojiSkinTone::{Default,Light,MediumLight,Medium,MediumDark,Dark}` | `Default` | Unicode's five modifiers, or the yellow default | sill FINDINGS F649 | proposed (2026-09-26) |
  | `launcher.web_search` | `WebSearch::{On,Off}` | `On` | "Search the web for …" as the last result | sill FINDINGS F650 | proposed (2026-09-26) |
  | `launcher.web_engine_url` | `String` | `https://duckduckgo.com/?q={query}` | an `https` or `http` address with `{query}`; anything else offers nothing | sill FINDINGS F650 | proposed (2026-09-26) |
  | `launcher.currency` | `CurrencyRates::{Off,Ecb}` | `Off` | `Ecb` fetches the ECB's daily reference rates at most once a day | sill FINDINGS F650 | proposed (2026-09-26) |

  All Advanced until Q303. Later, not registered: `launcher.web_suggestions` (off), and the
  section weights and caps (F645) if they should be settings.
- **Q302 (design/13)** §13.3.9 rows for the launcher v2 (proposed, from F645 and F654):
  "Sections | Top Hit, then Applications, Documents, Windows, Calculator, Settings, Clipboard,
  Emoji, System, Web; each capped (6, 5, 3, 1, 3, 3, 8, 3, 1) with Show More"; "Preview | a 360
  px column right of the results; Space toggles it while browsing (after Up or Down), ⌘Y always;
  → at the field's end shows it, a second → or ⌘→ enters it; Tab walks its controls; ← or Escape
  leave it, Escape again hides it"; "Row actions | every action has a chord shown beside it (H5):
  ⌘R reveal, ⌘C copy, ⌘B web, ⌘⌫ forget"; "Emoji | Tab to Emoji shows a grid of 8 columns,
  recent first; Enter copies".
- **Q303 (design/22 §5)** A Spotlight page (macOS has one: search results and privacy) with
  `launcher.clipboard_history`, `launcher.web_search` and `launcher.emoji_skin_tone` Basic: the
  history is a privacy choice a person should find in Settings, not only in the file.
- **G230 (shell-host / KWin)** Whether headless KWin 6.7 offers `ext_data_control_manager_v1` or
  `zwlr_data_control_manager_v1` is unverified; lane e reports it, and if neither, the clipboard
  step of the real-input run is NOT PROVEN there (cosmic-comp offers both).

## M9 lane b: the clipboard history (2026-09-26)

- **F680 The service is a task and a thread per connection.** `clipboard/live.rs` (the task
  `spawn_with` starts) owns the `ClipState`, reduces every command and every event through
  `reduce_clipboard`, publishes with `send_if_modified`, and carries out the effects: `Connect`
  starts `clipboard/wire.rs`'s thread (`sill-clipboard`: its own connection, `ext` else `wlr` on
  the first `wl_seat`, a calloop loop over the connection and a channel of orders), `Disconnect`
  drops the orders' sender, which stops the loop and drops the connection, and `OfferSelection`
  is an order. Each connection has an epoch; the task drops whatever an older one says, so a
  late `Connected` from a thread already told to stop cannot publish a protocol while off.
  Nothing connects before the daemon's first policy (`Services::spawn` in a test stays passive).
- **F681 Reading never blocks the loop.** An accepted offer is received into a `std::io::pipe`
  whose read end is a level-triggered calloop source (one `read` per readiness, so no
  `O_NONBLOCK` and no new dependency), given up at 2 s; an entry offered back is written the same
  way, one page per writable readiness, given up at 5 s. So the service reading its own offer
  back (the compositor reports it as a new selection) is served and read by the same loop, and
  the reducer's "same content keeps its key" rule makes it one entry (proven live: one entry
  after offering back). Limits: text **1 MiB** (`content::TEXT_LIMIT`, proposed, no settings key),
  images the policy's `clipboard_image_max_mb`; past either the read stops (`Skipped::TooLarge`).
  Text is kept only as valid, non-blank UTF-8 (so X11's Latin-1 `STRING`/`TEXT` only when ASCII);
  an image only when its header gives a size (`image::ImageReader::into_dimensions`).
- **F682 What is followed.** Only `selection`: `zwlr` is bound at version 1, which has no
  primary selection; `ext` always sends `primary_selection`, whose offers are destroyed unread.
  `offer_verdict` runs on the announced types before any `receive`: a `SECRET_HINTS` type skips
  the offer on its presence (the live test and `dev/accept-clipboard.sh` step 3 use the new
  `examples/secret_source.rs`, an `ext` data-control source offering made-up text plus
  `x-kde-passwordManagerHint`, which refuses a relative socket path and `wayland-0`). Text types
  are compared without case (`text/plain;charset=UTF-8`); images are PNG, else JPEG.
- **F683 The protocol cosmic-comp offers.** The nested cosmic-comp (`wayland-1`) advertises both
  `ext_data_control_manager_v1` v1 and `zwlr_data_control_manager_v1` v2; the service binds
  **`ext`**, and `sill clipboard status` says `memory ext`.
- **F684 Turning history on keeps the selection there is**: binding the device makes the
  compositor send the current selection, which is read like any other (klipper does the same).
  `dev/accept-clipboard.sh` clears the nested selection before turning history back on for that
  reason. Whether the one at start should be skipped is Q320.
- **F685 Rows.** `clip_items`: a text row's title is its first non-blank line, trimmed, cut to
  80 characters with an ellipsis; its subtitle "2 min ago · 1,204 characters" ("Just now", "N
  min ago", "N h ago", "Yesterday", "N days ago"); an image row "Image", "Just now · 640×480 PNG",
  matched by "image", its short type and its MIME type. Text matches on its first 10 000
  characters. The score is `rank` with no frecency (clipboard use is never counted), plus the
  title-prefix boost; ties keep newest first. Id is the key; `Subject::Clip` carries an excerpt
  at `ExcerptLimit::PANE` (a private helper, since `excerpt_of` is lane d's), never the whole
  text. One action, "Forget" ⌘⌫. `query`: nothing for nothing typed unless asked alone
  (`Scope::Only(Clipboard)`), then newest first; a batch is always pushed unless cancelled.
- **F686 Reducer rules beyond the frozen table**: a smaller image cap drops kept images over it;
  turning history off also publishes `not_connected` at once; `Connected` while off publishes
  nothing; a cap of 0 keeps nothing.
- **F687 Logging**: only fault kinds (`sill: clipboard: a selection was not written in time:
  timed out`, a socket or registry error) and the missing-protocol line; never a type list, a
  length or content. The example is found by Cargo's auto-discovery (no `[[example]]` entry:
  `crates/sill-services/Cargo.toml` is not this lane's).
- **Q320 (sill / design/22)** Should the selection present when history turns on (sill start,
  or `off` to `memory`) be kept? Today it is (F684). macOS keeps nothing before the manager runs;
  skipping it would make "turn history on" never reach back.
- **Q321 (sill-services, frozen)** `clip_items(history, needle, matcher)` has no `now`, so it
  reads the clock for the age words; the pure `rows_at(.., now)` is what the tables test. At the
  next freeze: `clip_items(.., now: UnixTime)`.
- **Q322 (sill-ipc, frozen)** `ClipboardReport` has no count of skipped selections, so a skipped
  password-manager copy is only observable as "still N entries". A `skipped: u32` (a count, no
  kinds) would let `sill clipboard status` prove it directly.
- **G250** `dev/accept-clipboard.sh` step 4 (launcher row, Enter, `wl-paste`) is PENDING until lane
  d answers `sill debug launcher-rows` and `launcher-key`; `tests/clipboard_live.rs` proves the
  same path below the launcher (`ClipboardCommand::Offer`, then `wl-paste` reads the entry).
- **G251** The script and the live test assume nothing else copies on the nested compositor while
  they run (another lane's acceptance copying an emoji would change the counts).
- Verified: `cargo test -p sill-services --test clipboard_live -- --ignored` on the nested
  cosmic-comp: ok (`Ext`; text kept, marked copy skipped, offered back and pasted, image 3×2 kept,
  forget, clear, off). `dev/accept-clipboard.sh`: 5 PASS, 1 PENDING (step 4), exit 2.

## M9 lane a: Documents (2026-09-26)

Lane a of "The M9 fill-in" (owns `providers/files/**`, `launch/open.rs`, `tests/files_*.rs`,
`examples/{files_probe, file_manager_stub}.rs`, `dev/accept-launcher-files.sh`). The eight
`a-files` tests run un-ignored; no frozen signature, public field or variant changed. Private
files added under `providers/files/`: `bus.rs` (F107's `Handle::block_on`, else a
current-thread runtime), `program.rs` (a search program within a budget), `stat.rs`, `cursor.rs`,
`query_tests.rs`.

- **F660 `FilesProvider::query` as its doc says.** The text is trimmed; nothing typed asks no
  index, also when Files is asked alone (`Scope::Only`); `FilesChoice::Off` asks none and is
  checked before detection, so an `Off` policy never touches the bus. Otherwise one `FileAsk {
  text, under: home, limit: 40, budget: policy.files_budget }` (40: the section publishes at most
  20, and a hit the name match drops leaves room); `Hits` and `Partial` are pushed as one batch,
  `Unavailable` and a cancelled query push nothing. Pinned by `files/query_tests.rs` with a
  scripted index that records its asks.
- **F661 Detection is cached once per process, not per provider.** `FilesProvider`'s frozen
  fields have no slot for a cache, so `detect()` lives in a `static OnceLock` beside it; the
  daemon builds one provider, so the two are the same today (a backend installed after sill
  started is seen after a restart). Tracker is `NameHasOwner` or `ListActivatableNames` (never
  started by detecting); Baloo is `baloosearch6` on `PATH` and `balooctl6 status` (C locale) not
  saying "disabled" (enabled but not running counts: `baloosearch6` reads the index itself);
  `fd` and `plocate` are `PATH` only (Debian's `fdfind` is not looked for).
- **F662 Tracker, checked live in localsearch's own sandbox.** `localsearch test-sandbox
  --store-tmpdir --index-recursive-directories <scratch>` runs a private bus and a miner over one
  folder only; `tests/files_tracker_live.rs` (`#[ignore = "needs localsearch"]`) runs the probe
  inside it and gets `m9-report.pdf` (`~/Documents`, `application/pdf`) and the folder `m9 dir`
  (`inode/directory`). What it showed: the name `org.freedesktop.Tracker3.Miner.Files` still
  answers beside localsearch 3.11's `org.freedesktop.LocalSearch3`; `Endpoint.Query(s, h,
  a{sv})` takes the write end of a `std::io::pipe()` (no new dependency, no unsafe); the cursor
  format documented in `cursor.rs` (host-order i32 column count, types and end offsets, then
  NUL-ended strings, row after row) reads correctly. The SPARQL needs `GRAPH tracker:FileSystem`
  and `DISTINCT`: over the union graph every file came back eight times. The miner skips a
  tmpfs folder, so the test's scratch folder is under the target directory. Name only, a
  case-insensitive substring like `fd`, never `fts:match`; with no rank the newest come first.
  The budget is a timeout on the call and a deadline on the read; a call that outlives it is
  `Partial` with nothing.
- **F663 Content hits are dropped (the lane's call `hits.rs` asked for).** A row shows a name, so
  the name must be why it is there: Tracker is not asked for content, and a Baloo hit that
  matched by content or a word prefix only is dropped by the name match. Two indexes naming one
  path make one row.
- **F664 Ranking.** `rank_text` over the name with the row's frecency, +50 for a file modified in
  the last 7 days and +25 within 30 (a date in the future counts as today's); ties by title, then
  path. Icons `folder`, `image`, `file`; actions Reveal in Files ⌘R and Copy Path ⌘C.
- **F665 The walk.** `fd` and `plocate` run with no shell, the text one argument after `--`, the
  C locale; output is read on a thread while the caller watches the clock and the query's
  generation (10 ms tick), killed at the budget, the line the kill cut dropped. A program that
  exits 1 still answers what it printed (`fd` after an unreadable folder, `plocate` with no
  match). `plocate --limit` counts matches everywhere before the home filter, so a common name can
  fill the limit with system files; `fd` comes first in `Walk`. `fd` honours `.gitignore`, so a
  home kept as a dotfiles repository that ignores `*` would hide every file (not handled).
- **F666 Opening (`launch/open.rs`).** `xdg-open` in `systemd-run --user --scope --collect
  --unit=app-sill-open.<pid>-<n>.scope` when `$XDG_RUNTIME_DIR/systemd/private` exists, else in
  its own process group (also when `systemd-run` is missing); the token as
  `XDG_ACTIVATION_TOKEN` and `DESKTOP_STARTUP_ID`; the child reaped on the runtime; a failure logs
  its `io::ErrorKind` only. The socket check is needed because `systemd-run --user` without a
  reachable manager exits 1, as `xdg-open` does, so the failure cannot be told apart afterwards.
  **The app launch path has that gap** (`launch/run.rs`, not this lane's): it falls back only on
  `NotFound`, so a daemon given a scratch `XDG_RUNTIME_DIR` (every acceptance script) cannot
  launch an app at all. A reveal calls `FileManager1.ShowItems([uri], token)` when the name has
  an owner, else `xdg-open` on the folder. URIs are percent-encoded as GLib writes them
  (`tracker::file_uri`, shared with the SPARQL prefix); **the dock's `launch/dbus.rs`
  `show_in_files` builds its URI unencoded** (`file://` + the path as displayed), so a
  `.desktop` path with a space or `#` is a broken URI (not this lane's file).
- **F667 Reveal and an activatable file manager.** As frozen, a reveal needs an owner of
  `org.freedesktop.FileManager1`; on this machine the name is only activatable (Dolphin), and
  cosmic-files is started the same way, so a reveal usually opens the folder without selecting
  the file. Spotlight selects it. Proposed: also call `ShowItems` when the name is activatable
  (activation starts the file manager with the file selected). Not done, because the frozen doc
  says "has an owner".
- **F668 Auto on a machine where localsearch is installed but idle.** Here `detect()` finds
  Tracker (activatable, no owner), Baloo absent (disabled), `fd` and `plocate` present, so `Auto`
  (the daemon's default, F652) picks Tracker, and the first keystroke's `Query` **activates
  localsearch-3, which then indexes the home folder under its own settings**, while that ask
  answers nothing within 150 ms. That follows the frozen `Detected::tracker` doc ("has an owner or
  is activatable") and `pick`'s order, but it is a side effect a person did not ask for.
  Proposed for the brief owner: `Auto` prefers a running Tracker, then Baloo, then `fd`, and an
  activatable-only Tracker only when named (`files_backend = "tracker"`); that needs
  `Detected::tracker` to tell owned from activatable, a frozen-type change, so it is reported,
  not made. The acceptance never reaches it: its daemon's private bus activates nothing.
- **F669 Acceptance (`dev/accept-launcher-files.sh`, exit 2).** On the nested compositor, with a
  scratch HOME, runtime, config, data, state and cache, a private bus and
  `XDG_CURRENT_DESKTOP=X-Generic` (so `xdg-open` asks `xdg-mime`, which reads the scratch
  `mimeapps.list` naming a stub `.desktop` that records its argument, the token and its cgroup):
  (1) the provider lists `m9-report.pdf` under `~/Documents` for `m9-rep`: PASS; the daemon's
  `launcher-rows`: PENDING (lane d). (2) Enter opens it with the token, bare (no user manager in
  the scratch runtime): PASS; and inside `app-sill-open.<pid>-1.scope` when only `systemd-run`
  sees the real runtime folder: PASS. (3) ⌘R: the stub `FileManager1` records `ShowItems` with
  the file's URI and the token: PASS; with no file manager on the bus the folder opens: PASS.
  (4) `off`: the provider lists nothing: PASS; the daemon after `sill reload`: PENDING (lane d).
  (5) `detect()` here: `tracker=Present baloo=Absent fd=Present plocate=Present`: PASS; Tracker
  end to end in its sandbox: PASS. Once lane d answers rows, steps 1 and 4 check the daemon with
  no change to the script.
- **Q310 (design/20 §1.3) Is a file matched smart-case?** The launcher's `Needle` is smart case
  (a capital makes it case-sensitive), so typing `M9` asks every index case-insensitively and
  then drops `m9-report.pdf`. That is consistent with apps, but Spotlight matches file names
  without case. A row for §1.3 ("file names match without case") would need `Needle` to take a
  case mode (a sill-launcher change, frozen).

### M9 lane a follow-ups (2026-09-26)

The coordinator authorised the frozen-type change below and `launch/{run,dbus}.rs` for this
round. F661 and F665 stand as recorded.

- **F670 `Auto` never starts an indexer (fixes F668).** `choice::Detected.{tracker, baloo}` are
  now `choice::IndexState::{Running, Dormant, Absent}` (was `Presence`): Tracker is `Running`
  when `org.freedesktop.Tracker3.Miner.Files` has an owner and `Dormant` when it is only
  activatable; Baloo is `Running` when `balooctl6 status` says "is running", `Dormant` when it
  is enabled but "is not running" (or says something unrecognised), `Absent` when disabled or
  not installed. The rule in `pick`: **`Auto` asks a running Tracker, else a running Baloo, else
  `fd`, else `plocate`; it never asks a dormant indexer.** A backend named in the setting
  (`tracker`, `baloo`) is asked while it is `Running` or `Dormant`, so `files_backend =
  "tracker"` may still start localsearch: the person chose it. Pinned by the 15-row table
  `auto_never_starts_an_indexer_and_a_named_backend_is_never_substituted`. Call sites updated:
  `baloo.rs`, `tracker.rs`, `files_probe`, `tests/files_{walk,tracker}_live.rs`. On this machine
  `detect()` is now `tracker=Dormant baloo=Absent fd=Present plocate=Present`, so `Auto` walks
  with `fd`. In localsearch's sandbox the probe sees the miner `Dormant` or `Running` depending
  on timing; the named backend finds the files either way. The owner/activatable check is one
  helper, `providers::files::bus::name_state` (`NameState::{Owned, Activatable, Missing}`),
  shared with the reveal (F672).
- **Q311 (design/22) The `launcher.files_backend` row (Q301) changes** its Range / Alt text to:
  "auto asks a running Tracker 3, else a running Baloo, else fd/plocate; it never starts an
  indexer. tracker or baloo asks that index and may start it; a named backend that is missing
  searches nothing".
- **F671 Launching without a reachable user manager (fixes F666's gap).** `launch/run.rs`'s
  `start` now asks `open::user_manager()` (the same `$XDG_RUNTIME_DIR/systemd/private` check
  `open.rs` uses, shared, not copied) and, when it is `Missing`, runs the command after
  `systemd-run`'s `--` in its own process group straight away; the `NotFound` fallback stays for
  a manager that is reachable but has no `systemd-run`. Test
  `an_unreachable_user_manager_runs_the_bare_command` (with `/bin/false` in `systemd-run`'s
  place, only the bare command can succeed). A daemon under a scratch runtime folder can now
  launch apps. `launch/dbus.rs`'s `show_in_files` percent-encodes the `.desktop` file's URI with
  `tracker::file_uri`; table test with a space (`My%20App.desktop`) and a `#` (`c%23-ide`).
- **F672 A reveal asks an activatable file manager too (fixes F667).** `ShowItems` is called when
  `org.freedesktop.FileManager1` is owned or activatable (activation starts it with the file
  selected, as Spotlight's reveal does); only a missing name falls back to `xdg-open` on the
  folder. The acceptance checks all three: the stub running, the name missing, and the stub as a
  service file on the private bus (its own config, `dev/private-bus.conf` plus one scratch
  `servicedir`, reloaded with `ReloadConfig`), started by the call.
- **F673 File names match without case (answers Q310 inside the provider).** `hits::file_items`
  re-parses the needle lower-cased (`without_case`), and a lower-case needle is case-insensitive
  under the launcher's smart case; nucleo's operators have no case, so `^M9` still anchors. No
  sill-launcher change. `M9`, `m9-REP`, `Return` and `^M9` find their files (table test); the
  Tracker sandbox test now asks `M9`, and the acceptance asks `M9-REP`.
- **F674 `fdfind`.** `fd` is looked for as `fd`, then `fdfind` (Debian and Ubuntu install it
  under that name), both in `detect()` and when `WalkIndex` runs it
  (`program::{FD_NAMES, first_on_path}`, table test).
- **F675 Acceptance after the follow-ups (exit 2).** Step 1 PASS twice (`m9-rep` and `M9-REP`),
  PENDING for the daemon (lane d); step 2 PASS bare and in `app-sill-open.<pid>-1.scope`; step 3
  PASS three times (running, missing, activatable); step 4 PASS, PENDING for the daemon; step 5
  PASS (`tracker=Dormant baloo=Absent fd=Present plocate=Present`; Tracker in its sandbox). The
  full suite once failed `notification_center_render::a_press_on_a_stack_opens_it_and_a_lone_
  cards_close_dismisses_it` (sill-surfaces, not touched here) while the machine was loaded; it
  passed three times alone and in the `--no-fail-fast` rerun (1034 passed, 0 failed).

## M9 lane c

Emoji, the web fallback and currency (FINDINGS "M9 freeze" F649/F650), against `m9-freeze`
(f17ea53). Numbers from F700, Q330, G260.

- **F700 Emoji** (`providers/emoji/{table,search}.rs`). `table::emoji_table()` builds once
  (`OnceLock`) from `emojis::iter()` (default-tone glyphs only, Unicode order) joined with
  `parse_keywords(KEYWORDS_TSV)`; `Tones::Takes` iff `Emoji::skin_tone().is_some()`. `search::
  emoji_items` checks each entry's name, then (outside a `:` search) its keywords, then its
  shortcodes with nucleo's `Needle::score`, the FIRST of these that matches deciding the entry's
  `MatchKind`; the final `Score` is `matched_score * 100_000 + kind_rank * 10_000 +
  reverse_position`, so the fuzzy match quality dominates (a strong keyword or shortcode hit is
  never beaten by a scattered, low-quality name match — considered and rejected: multiplying by
  `MatchKind` first, which let a weak subsequence match on a long CLDR name from crowding out an
  exact shortcode hit, e.g. `:tada` against 🎉's own shortcode), `MatchKind` only breaks a tie,
  and the table's own (Unicode) position breaks what is left, so no two rows ever tie into
  `reduce_results`'s title sort (`results/model.rs`). Nothing typed under the Emoji filter
  (`Scope::Only(Emoji)`) scores by frecency first, then position, the same way. A row's title is
  the glyph itself (`toned`, `emojis::Emoji::with_skin_tone` mapped from this crate's `SkinTone`);
  its `ItemId`/frecency key is always the default-tone glyph (F650). `EmojiProvider::query`
  requires two or more typed characters, or any non-empty text after a leading `:`, matching the
  module's own doc; one bare character or a bare `:` lists nothing, whatever the scope. The limit
  passed to `emoji_items` is `SectionPlan::default_plan().ceiling` (20, F645), not the grid's
  display cap of 8 — the cap is lane d's `place`/`shown`, not a provider concern.
- **F701 The web fallback** (`providers/web.rs`). `search_url` keeps RFC 3986 unreserved bytes
  (letters, digits, `-._~`) literal and percent-encodes everything else, including space, as
  `%20` (`percent_encoding::NON_ALPHANUMERIC` with those four removed from the encode set); it
  refuses a template with no `{query}` or not `https`/`http`, checked in that order (a `file:`
  template with `{query}` is `NotWeb`, not `NoPlaceholder`). `web_item`'s title is
  `Search the web for \u{201c}{text}\u{201d}` (literal curly quotes, matching the verify step's
  wording exactly); its subtitle and `WebFacts::engine` are the template's host
  (`host_of`, a plain split on `/`, `?`, `#` after the scheme). `WebProvider::query` reads
  `LauncherPolicy::web` fresh each call (a `watch::Receiver`, so a settings change is live next
  keystroke) and offers nothing for empty text or an off/refusing template.
- **F702 Currency** (`providers/calculator/{detect,eval,rates,fetch}.rs`). Three layers:
  - `rates::parse_ecb` is a small attribute scan (`<Cube currency="USD" rate="1.1000"/>`, single
    or double quoted), not an XML parser, gated on a namespace fragment only the ECB's own file
    carries (`ecb.int/vocabulary`) so `parse_ecb("<html/>")` is `NotEcb`, not an empty table;
    `relative_to_euro` is `1 / per_euro[code]`. `rates::ExchangeRates` wraps a `Rates` snapshot as
    `fend_core::ExchangeRateFnV2`, installed by `eval::evaluate` only when it is given `Some`
    rates — fend already knows the ISO codes as lazily-evaluated currency units (its
    `units/builtin.rs`), so nothing here declares them; only the exchange rate needs supplying.
  - `detect::looks_like_expression` and `word_is_known` gained an `Option<&Rates>` parameter
    (both were pure functions this crate owns outright, not part of the interface freeze, so
    widening them needed no cross-lane coordination): a currency code counts as a known unit word
    only when `rates` is `Some` and has it, so `"10 usd to eur"` is a plain search term while
    `launcher.currency = "off"` and an expression once ECB has given a table. `eval::evaluate`
    gained the same parameter and installs the exchange-rate handler only then.
  - `fetch.rs` (new) is the only effectful part: `due` reads the cache file's own mtime (never an
    in-memory field — `CalculatorProvider`'s fields are frozen at `policy`/`rates`, and this
    avoids widening them) and says a fetch is worth trying at all, missing or damaged file
    included; `refresh_if_due` is fire-and-forget (`tokio::spawn`, guarded by a process-wide
    `AtomicBool` so two keystrokes in a row never start a second fetch), runs `curl --fail
    --silent --show-error --output <path>.xml.tmp <ECB_DAILY_URL>` and renames the temp file in
    on success only, so a reader never sees a half-written file. `CalculatorProvider::ecb_rates`
    (private) is what `query` calls every time `launcher.currency = "ecb"`: it reloads the cache
    from disk fresh (a local file read of a few KB, well inside the 50 ms budget, never the
    network) so a pre-seeded cache converts on the very first query with no fetch at all, writes
    it into the shared `SharedRates` (so other readers see the same snapshot), and only then asks
    `refresh_if_due` whether the file itself is stale enough to refetch. `off` calls none of this
    (`ecb_rates` returns `None` before touching the policy's `.currency` a second time, let alone
    the disk).
- **F703 The emoji paint bench** (`crates/sill-surfaces/examples/emoji_paint_bench.rs`, F656's
  open risk). 300 real `.ds-emoji-text` cells (quire m9-parts, corrected by the coordinator
  mid-wave: the colour-emoji class is `.ds-emoji-text`, not `.ds-emoji` — that root is
  `AnimatedEmoji`'s) in a scrollable flex grid inside a `ds::Ds { material: Material::Sheet }`
  root, built with `ds_native::Harness`, scrolled with `Harness::wheel` and `Harness::advance`
  for 180 simulated 60 Hz frames (3 s), each frame's `Harness::render()` timed with `Instant`.
  **vello_cpu** (release build — a debug build's numbers, p50 151 ms, are an artifact of no
  optimisation and not reported as the finding): **p50 6.89 ms, p95 9.57 ms, max 12.95 ms**, all
  comfortably inside the 16.67 ms/frame budget for 60 Hz on this machine. Sent to the main
  session ahead of the rest of this lane's work, since lane d's grid depends on it.
  **vello_hybrid was not measured**; see Q330.
- **The six ignored tests are un-ignored** (`table::tests::keywords_and_names_come_through`,
  `search::tests::{names_keywords_and_shortcodes_all_find_an_emoji, recent_emoji_lead_the_empty_
  grid, a_tone_applies_only_where_it_can}`, `web::tests::the_query_is_percent_encoded_into_the_
  template`, `rates::tests::the_daily_file_becomes_rates_per_euro`) and pass, plus new
  table-driven tests for the currency gating (`detect.rs`, `eval.rs`) and the fetch throttle
  (`fetch.rs`), none of which touch the network (a `Rates` value is built by hand in every test;
  `fetch::due`'s own tests use a file this process wrote, never `curl`).
- **Q330 (quire)** `ds_native::Harness` paints only through `anyrender_vello_cpu`
  (`ds-native/src/headless.rs` hardcodes `VelloCpuImageRenderer`); there is no public path to
  paint a `Harness`'s document through `anyrender_vello_hybrid` instead. The existing hybrid
  example (`ds-native/examples/emoji_probe_hybrid.rs`) does not answer this: it hand-builds an
  anyrender scene and drives a raw wgpu device directly, bypassing the DOM, layout and `Harness`
  entirely, so it cannot time real markup (300 real `.ds-emoji-text` cells, real cascade, real
  scroll). Proposed: a `Harness` method (behind a dev-feature, mirroring `pdf-thumb`'s pattern)
  that paints the current frame through `anyrender_vello_hybrid` on an offscreen wgpu texture
  instead of `anyrender_vello_cpu`, returning the same `image::RgbaImage` `render()` does, so a
  caller times it the same way. Blocked in this wave on `crates/sill-surfaces/Cargo.toml`, which
  the M9 fill-in gives to lane d (the `pdf-thumb` feature only) — lane c did not touch it.
- **F704 A failed fetch backs off for an hour, and the calculator only reads for it when the
  text could need it** (coordinator follow-up on F702, before merge). Two things F702 missed:
  - `fetch::due` counted only the cache's own age, so a `curl` that fails (offline, blocked, a
    proxy) left the cache exactly as stale as it always was — every following keystroke with
    `launcher.currency = "ecb"` saw it as due and started another `curl` the moment the last one
    ended, which is "on a keystroke" in every sense that matters even though no single call
    blocks one. `due` gained a second input, the last attempt (`fetch::Attempt { at, outcome }`,
    a sibling stamp file next to the cache, `ecb-rates.attempt`, `<unix seconds> success|
    failure`): a failed attempt holds off the next one for an hour regardless of the cache's own
    age; a successful one still holds off for a day, as before. `fetch::fetch_into` itself had a
    bug in the same family: it returned `Ok(())` even when `curl`'s exit status was non-zero (it
    only checked the status to decide whether to rename the temp file in), so a failure was never
    distinguishable from a success by its caller — fixed to return `Err` on a non-zero exit, the
    only way `refresh_if_due` can tell the two apart to write the right stamp. `due` stayed pure
    and table-tested (`fetch::tests`, one table: missing cache and no attempt; a failed attempt
    10 minutes ago; a failed attempt 2 hours ago; a fresh cache; a day-old cache) by taking
    `Option<UnixTime>` (the cache's own `mtime`) and `Option<Attempt>` rather than a `Path` — the
    reads themselves (`mtime`, `read_attempt`) moved to `refresh_if_due`, the effectful edge.
  - `CalculatorProvider::ecb_rates` (the disk read plus the due check) ran on every query while
    `currency = "ecb"`, whatever was typed — `"2 + 2"` loaded the cache exactly as `"10 usd to
    eur"` did. `detect::mentions_possible_currency` (new, pure, table-tested) is a cheap,
    table-free shape check — any word, once a leading number is stripped as `word_is_known`
    already strips one, that is exactly three bare letters — and `query` now calls `ecb_rates`
    only when it says yes. It is deliberately loose (`"ftp"` or `"www"` also pass it): getting
    this wrong only costs an extra, harmless disk read, never a network call, since the network
    throttle is `due`'s alone.

## M9 lane d: sections, the preview pane, the rows, the debug keys (2026-09-27)

Against `m9-freeze` (f17ea53), then master b9802d5 (lanes a, b and c merged) and quire master
58fc3e48 (m9-parts and m9-palette). Numbers from F720, Q340, G270.

- **F720 Sections.** `place` and `shown` are filled as their docs say (Top Hit by `weighted`
  score among non-zero weights, earlier row on a tie, never repeated below; nothing typed:
  Actions, then the `recent` most frecent non-System rows, Clipboard and Web never Recent;
  `shown` caps each section at its provider's cap, Top Hit/Actions/Recent never). The runner
  (`results/arrange.rs`, `run.rs`) now publishes `place`'s items with their spans in
  `Results::sections`; the M4 grouping (F114's `grouped`, `actions_then_recent`, `group_cap`) is
  retired. A filtered query (`Scope::Only`) publishes that provider's rows alone under its own
  section, **uncapped by the ceiling** (the provider bounds its answer; the emoji provider's
  recent set is 20). The panel reads the spans (`launcher/view.rs`); a runner that publishes no
  covering spans (a test fake) is placed on the panel's side with no frecency.
- **F721 The preview.** `preview_for`: PDF for `application/pdf`; `Image` for PNG, JPEG, SVG,
  WebP, GIF, BMP; `TextFile` for `text/*` and 14 `application/*` source and config types
  (json, xml, toml, yaml, javascript, shell, sql, desktop, perl, python, ruby); `File` for the
  rest and folders. `excerpt_of`: a NUL in the first 1024 bytes is binary; lossy UTF-8, cut at
  `lines` then `chars`, trailing whitespace trimmed. `read_excerpt` reads at most 16 KiB
  (`Read::take`), `NotFound` is `Missing`, anything else `Unreadable`; the pane calls it on a
  thread of its own and shows the answer only for the path it asked (`preview/load.rs`).
- **F722 quire's parts, all real.** The pane is quire's `PreviewPane` as `CommandPalette { aside
  }` (sill's frozen `PreviewPane` wraps `LauncherPane`, which maps a `Preview` to `PaneContent`,
  a PDF's page from `ds_native::use_pdf_page(Some(path), PANE_MEDIA)` asked only while the
  preview is a PDF so one pane stays mounted); it plays out through `shown: Hidden` and is
  dropped at `on_hidden`. Show More/Show Less is `PaletteGroup::with_action` (a cursor stop after
  the section's last row, so `selected` counts it: the view's `Line::More`); Emoji is
  `PaletteGroup::grid` (`EMOJI_COLUMNS` 8, `EMOJI_CELL` 56, `.ds-emoji-text`); file rows are
  `RowShape::File` (a picture file's own pixels as `thumb`, `~/…` as `location`, the mail
  list's time as `modified`) and clipboard rows `RowShape::Clip` (two lines of text, or the PNG
  from `use_clipboard`); the providers' glyphs are `Clipboard`, `Smile`, `Globe`. The frozen
  `palette_sections` stays as plain lists (a picture without handlers); the panel draws the new
  `palette_groups`. `pdf-thumb` is on in `sill-surfaces/Cargo.toml` (`cargo deny` ok).
- **F723 The panel widens.** The layer surface is now `PANEL_ROOM_WIDTH` (680 + 360 = 1040) plus
  the shadow's room for good; the 680 card sits centred in it (`CARD_INSET` 180) and the body
  widens to 1040 while the aside is there, so the card grows by half the pane on each side and
  its middle never moves. The blur region is still the card element (the aside is inside it);
  while the aside is there the panel takes the whole room (`PanelHit::Whole`) and the catcher's
  hole is the room; otherwise the hole is the measured card, 680 wide. `panel_rect` is now the
  room (tests updated).
- **F724 Keys.** quire's key claim (Q299) takes, before the palette: Space while browsing (the
  field types nothing), → with the caret `AtEnd` (`Inside` and `Unknown` pass: → moves the
  caret), ⌘Y and ⌘→, Tab/Shift+Tab/← in the pane, Escape while the pane shows or has the
  keyboard, and any chord the selected row's actions carry (`action_for_chord`: ⌘R reveals, ⌘C
  copies, ⌘B searches, ⌘⌫ forgets) straight from the results. The pane never takes the
  keyboard: its focus is the machine's `PaneFocus::Pane(n)`, drawn by quire as `focused`, and
  Enter there runs that action (`pane_choice`). `onkey` keeps Tab's filter, Ctrl+K and the Up/Down
  that make Space Quick Look. Tab sends `SearchCommand::Filtered`, and typing under a filter
  stays filtered; the panel shows only the filter's rows while the answer is on its way. Tab's
  ring: with text typed, the providers that showed a row; with nothing typed, every provider (a
  provider asked alone may list what it has for nothing typed: the emoji grid). A forgotten
  clipboard entry asks the query again. The caret needs quire's `HostCaret`: the panel calls
  `ds_native::focus::provide()` itself (the daemon's roots carry `FOCUS` only; G270).
- **F725 The debug link.** `LauncherLink` gains two private channels and four methods
  (`press`, `subscribe_keys`, `show_rows`, `rows`; additive, no frozen field or signature
  changed); `launcher/debug_link.rs` holds the surface-side `LinkKey`/`ShownRows` (sill-surfaces
  does not depend on sill-ipc) and `launcher_debug.rs` maps them. The panel publishes its rows
  after each render while open (clipboard titles already `<clipboard>`, so no clipboard text
  leaves the panel) and `None` when it closes. `launcher-rows` lists a section's header action
  as a last line "Show More"/"Show Less", and `selected` counts it, as the palette's cursor does.
  `launcher-key right` cannot see the field's caret and acts as → at the end (G271).
- **F726 `LauncherProviders::from_services` builds all nine providers** (M9's `activate` reads the
  row alone; their policy there is the default and never queried), so a picked file, clipboard,
  emoji or web row does what its provider says rather than nothing.
- **F727 `grid_step` is quire's**, wrapping Left/Right across rows; the freeze's test that
  clamped Right at 7 now expects 8 (changed on the coordinator's authority, as quire answered
  Q291 with the character viewer's wrap).
- **F728 PDF rasters under a held key.** `tests/launcher_pdf_rasters.rs` (its own binary: the
  counter is per process): the pane shown, Down through three PDFs one render apart, then the
  queue drained: 1 raster (`ds_native::pdf_thumb_rasters`). The fixtures are heavy on purpose
  (6000 stroked curves), since a trivial page rasterises inside a frame and proves nothing.
- **F729 Acceptance** (`dev/accept-launcher-v2.sh`, nested cosmic-comp, daemon `--features
  debug`, scratch HOME, `files_backend = "fd"`, private bus, own runtime dir), after merging
  master b9802d5 and adopting quire 58fc3e48: every step PASS with lanes a and c's real rows.
  (1) 10 warm openings `open_to_frame_ms` p50 108.8, p95 156.6, max 156.6 (earlier runs on this
  host under load: p50 108-228; master's alternated run in F167, p50 120-130); (2) `--query m9`: Top Hit,
  Documents, Web; (3) PDF preview (`dev/out/launcher-v2-preview-pdf.png`); (4) the PNG behind
  Show More, `image`, Space hides (`launcher-v2-preview-image.png`); (5) ⌘Y while typing, → into
  the pane (`launcher-v2-pane-focus.png`: the ring on Reveal in Files), Escape three times closes
  one level each; (6) Documents hides 3, Enter on Show More reveals them; (7) Shift+Tab twice
  with nothing typed: the emoji grid, 20 cells (`launcher-v2-emoji.png`); (8) 5 s idle: 0 frames.
  (C) The caret under sill, with real keys (`wtype` on the nested socket only): after `--query
  m9` a real → leaves the pane hidden (the caret was at the start, Q341) and End then → opens
  it (`pdf`), the query still `m9`: the claim reads `AtEnd`/`Inside`, not `Unknown`.
  The demo over the fakes' fixtures (`launcher-v2-*-fixtures.png`: pdf, image, text, show-more,
  pane-focus, emoji) passes too. The harness tests (`tests/launcher_preview.rs`) prove the same
  over fixtures, with real Space, → (after End) and Escape through the claim.
- **F730 Lanes a and b's scripts.** Rerun with this branch's rows and keys: every step passes but
  the one that reads `launcher-rows` in each, which looks for a section titled `Documents`
  (`accept-launcher-files.sh` step 1) or `Clipboard` (`accept-clipboard.sh` step 4); a single
  match is the Top Hit, which F645 never repeats below. A copy of lane b's script that accepts
  Top Hit passes step 4 end to end (one row, `launcher-key enter`, `wl-paste` reads the entry).
  Both checks belong to their lanes' owners (G272).

### Gaps found in lane d

- **Q340 (quire) A controlled selection moved by the caller does not scroll into view.** With
  `selected` driven by `sill debug launcher-key down`, the selected row can sit below the card's
  visible rows (`launcher-v2-preview-image.png`: m9-shot.png selected, off the card). Real Up and
  Down are the palette's own; whether they scroll is to be checked in lane e.
- **Q341 (quire/Blitz) A field handed its first value keeps its caret at the start.** After
  `sill launcher open --query m9` the caret reads `Inside`, so → moves it rather than opening
  the pane until End; macOS puts it at the end.
- **Q342 (quire) Wide gaps between words and between a letter and a digit** in every capture
  on this host (`m 9 -report.pdf`, `Show   More`, quire's own `Reveal   in   Files`), and a
  row's time runs into its chord (`Mon⌘R`). Seen in quire's parts, not sill's markup.
- **G270 (sill daemon) Roots carry `FOCUS` but not `CARET`.** `sill_surfaces::CARET` is exported
  for `daemon.rs`'s `surface_root` (not lane d's file); the launcher provides it itself.
- **G271 `launcher-key right` is → at the end.** A debug key never reaches the field, so it
  cannot read the caret; the acceptance relies on that (step 5), real input on the claim.
- **G272 Lanes a and b's `launcher-rows` checks** expect their section title for a single match
  (F730).
- **G273 D0 (`use_detail`, Q297) is still missing:** the row and pane moments call nothing; quire's
  pane plays its own entrance and exit.

## M9 lane e: launcher v2 under real keyboard input (2026-09-27)

Against master 17f52ac (lanes a-d merged). Owns `dev/accept-launcher-v2-real-input.sh`, its
entries in `docs/manual-checks.md`, and (authorised, G272) the section checks in
`dev/accept-launcher-files.sh` step 1 and `dev/accept-clipboard.sh` step 4. No source file
changed. Numbers from F740, Q350, G280.

- **F740 The real-input acceptance** (`dev/accept-launcher-v2-real-input.sh`, exit 1: one
  failure, F741). On shell-host's headless KWin (1920x1080, `kwin-virtual.sh start`, stopped at
  the end), one `fake-input.sh` client for the whole run (G50), the daemon (`--features debug`)
  given KWin's socket as an absolute path, KWin's private bus (never exported to the script:
  G52), its own `XDG_RUNTIME_DIR` (under `/tmp`: the agent socket's path must fit 108 bytes) and
  a scratch HOME, config, data, state and cache with `files_backend = "fd"`. The script refuses
  a socket that is not `wayland-kwinvirt-*` and a `.kwin-env` that is already there (another
  run's KWin is never borrowed or stopped). Every key is real; `launcher-rows` and `capture`
  only look.
  (1) PASS: typing `m9-report` lists `m9-report.pdf` as Top Hit; Backspace to `m9`, Down, Space:
  the preview shows (`text_file` for `m9-list-02.txt`) and the query stays `m9`; Space again
  hides it; typing `-rep`, Ctrl+Y (⌘Y): `pdf` while typing. (2) PASS: → shows the pane, a
  second → puts the focus ring on its first button (the captures differ only inside the pane,
  x 735-883); Tab moves the ring (735-1010, 342-378), a second Tab brings it back (the first and
  third captures are identical; two buttons, it wraps), the headings unchanged (Tab is not the
  provider filter there); Escape 1: the ring goes and the pane stays (the capture equals the
  one before entering); Escape 2: the pane hides, the launcher stays; Escape 3: it closes.
  (3) PASS: Ctrl+R (⌘R) on the file row: the stub file manager on KWin's bus records
  `ShowItems` with the file's `file://` URI and the token `kwin-1` (a real xdg-activation
  token), and the launcher closes. (4) FAIL (F741): Shift+Tab twice with nothing typed: the
  Emoji grid alone, 20 cells; Down 0→8, Down 8→16, **Right 16→16 (the preview opened
  instead)**, Left 16→15, Up 15→7; Enter copies cell 7 (`wl-paste` on KWin's socket reads the
  selected glyph) and closes. (5) PASS: see F743; one clipboard row (`<clipboard>`, Top Hit) for
  `lighthouse`, selected, real Enter, `wl-paste` reads the made-up entry back.
- **F741 (lane d) → never moves in the emoji grid.** `PanelCtx::claim`
  (`surfaces/launcher/pane_keys.rs`) takes every `ArrowRight` as `PreviewKey::Right(caret)`
  before the palette; with nothing typed the caret is always `AtEnd`, so → shows the emoji's
  preview (and a second → would enter it) and quire's grid never gets it, while ←, ↑ and ↓ do
  (← is claimed only in the pane). Repro: `dev/accept-launcher-v2-real-input.sh` step 4, or by
  hand: open, Shift+Tab twice, Down, → (`launcher-rows`: `selected` unchanged, `preview` `shown
  emoji`). Proposed: while the selection is in a grid group, → (and ←) pass to the palette, and
  the preview is ⌘Y/Space only there, as in the macOS character viewer. `launcher-key right`
  cannot show this (G271: it is always the preview's →).
- **F742 (lane d, a question) `↵` stays on the first row.** `sections_view.rs`/`entries.rs`
  `trail` gives `↵` to the first row of the list and `⌘R` (or `⌃K`) to the rest, whatever is
  selected; its doc says "`↵` on the row Enter runs", and Enter runs the selected row. After a
  real Down the highlighted row shows `⌘R` and the unhighlighted first row still `↵` (the
  script's `preview-space.png`: `m9-list-02.txt` selected, `m9-list-01.txt` with `↵`).
  Spotlight shows the return glyph on the selected row. Kept for lane d: whether `first`
  should be `selected`.
- **F743 G230 answered: headless KWin 6.7 offers `ext_data_control_manager_v1` v1** (and no
  `zwlr_data_control_manager_v1`); `sill clipboard status` says `memory ext`. So step 5 is
  proven on KWin, not NOT PROVEN. wl-clipboard 2.2.1 uses the same protocol, so `wl-copy` and
  `wl-paste` need no focus there. An emoji copied by Enter is read back into the history like
  any other selection (the status read `memory ext 1` after step 4), as the service reads its
  own offers (F681).
- **F744 Q340 stands under real keys.** With Documents filtered by a real Tab (24 rows), eleven
  real Downs select row 11 and the card still shows rows 0-7: no row carries the selection tint
  (0 px against 7484 on row 0; `q340-down.png`); eleven real Ups bring it back into view only
  because row 0 never left. So the palette's own Up and Down do not scroll the selection into
  view either; it is quire's (`CommandPalette`'s list), not the debug link's. The script records
  it as `Q340 OPEN|CLOSED` and does not fail on it.
- **F745 Q341 stands under real keys.** After `sill launcher open --query m9` a real `x` makes
  the query `xm9`: the caret starts at the beginning. A real → then leaves the pane hidden (it
  moved the caret), and real End then → shows it (`pdf`, the query still `m9`), as lane d saw
  with `wtype` on the nested compositor (F729 (C)). Recorded as `Q341 OPEN|CLOSED`.
- **F746 G272 fixed: lanes a and b's section checks.** `accept-launcher-files.sh` step 1 reads
  the rows as JSON and passes when `m9-report.pdf` is listed once, under Top Hit or Documents
  (it is the Top Hit). `accept-clipboard.sh` step 4 finds the one `<clipboard>` row under Top
  Hit or Clipboard and moves `launcher-key down` onto it before Enter: since lane c merged,
  "lantern" makes the emoji 🏮 the Top Hit and the clipboard row sits under Clipboard, so Enter
  on the first row would copy the lantern, not the entry (F730's passing copy predates that).
  Rerun on the nested cosmic-comp (wayland-1 pinged first, reused): `accept-launcher-files.sh`
  12 PASS, exit 0 (step 1: "the daemon lists m9-report.pdf under Top Hit"; step 5
  `tracker=Dormant baloo=Absent fd=Present plocate=Present`, Tracker in its sandbox);
  `accept-clipboard.sh` 6 PASS, exit 0 (step 4: "clipboard rows: 1, under Clipboard",
  `wl-paste` read the offered entry).
- **Q350 (lane c) Scattered emoji matches.** `--query lantern` lists, after 🏮 and 🎃, the flags
  🇲🇵 🇳🇱 🇧🇶 🇹🇫: "lantern" is a subsequence of "fLAg: Northern mariana islands", "flAg:
  NeTherlands" and so on. F700 lets the match quality rank them but keeps them; Spotlight shows
  only word-prefix matches. Proposed: an emoji name or keyword matches on word prefixes (or
  under a minimum score), so a flag needs "flag" or its country's word.
- **G280 `accept-launcher-files.sh` step 4 checks only for a `Documents` title.** With
  `files_backend = "off"`, a file row that were the Top Hit would still pass it (the same
  shape as G272). It passes today (no file row at all); tightening it (no `m9-report.pdf` in any
  section) is lane a's, not authorised here.
- **G281 The script's fake-input client builds shell-host's tool with the caller's HOME.**
  `fake-input.sh` runs `cargo build`, which needs rustup's default toolchain under `$HOME`; the
  script gives the scratch HOME to the daemon only (a client started with the scratch HOME fails
  with "rustup could not choose a version of cargo").

## M9 fixes (2026-09-27)

Against master 47d606f (every M9 lane merged). Fixes F741, F742, Q350 and G280; each has a
table or harness test that fails without its fix. Numbers from F760.

- **F760 F741 fixed: ← and → move the emoji grid.** design/13 §13.3.9 gives the grid "2D arrow
  keys", so on a cell of the Emoji section the claim leaves ← and → to quire's palette, which
  moves the cell (`claim_keys::pane_key`, a pure table of the claim's keys; `View::layout` says
  whether the selected line is a grid cell: an Emoji-section row, not an emoji Top Hit and not
  the Emoji header's "Show More"). The pane there is ⌘Y's, ⌘→'s and Space's, as in the
  reference's character viewer; a grid move feeds the pane machine `Moved` (`field_key`), so
  Space after it is Quick Look and a shown pane follows the cell. With the pane holding the
  keyboard, ← and → stay the pane's. Tests: `claim_keys` (19 cases; the grid rows fail with the
  old claim), `view_tests::only_an_emoji_sections_cells_are_grid_lines`, and the harness's
  `real_left_and_right_move_the_emoji_grid_and_command_y_previews` (real →, ←, Space, ⌘Y, ⌘→;
  without the fix, "→ moved to the next cell" reads 0, not 1).
- **F761 F742 fixed: `↵` trails the selected row.** The row Enter runs is the one the palette's
  cursor is on, so `trail` gives `↵` to that line (`entries::Cursor`, counted as the palette
  counts its stops: each row, each emoji cell, a header's "Show More"); every other row keeps its
  chord (`⌘R`, `⌃K`) or note. The panel passes `session.keys.selected` (`Draw::cursor`), the M4
  `ResultsList` its `selected` (`entries::palette_groups_at`); the frozen `palette_sections` and
  M4 `palette_groups` keep their signatures with the cursor on the first line. On "Show More" or
  an emoji cell no row trails `↵`. Tests: `sections_view_tests::the_return_glyph_follows_the_cursor`
  and `entries::tests::the_return_glyph_moves_with_the_selection`. The real-input run's
  `preview-space.png` now shows `↵` on the selected `m9-list-02.txt` and `⌘R` on the Top Hit.
- **F762 Q350 fixed: emoji match on word prefixes.** An emoji is listed only when every typed word
  begins a word of its name, keywords (not after `:`) or shortcodes (`emoji/words.rs`; words
  split on anything not a letter or digit, compared without case), and nucleo's score still
  ranks what is left. "lantern" lists 🏮 then 🎃 (jack-o-lantern) and nothing else; "neth" still
  finds 🇳🇱; "thumbs" → 👍 first and `:tada` → 🎉 hold. The words may come from different fields,
  but a row is scored per field as before, so a query whose words are spread over the name and a
  keyword with no single field matching it fuzzily is dropped (none seen). Tests:
  `words::tests` (12 cases) and `search::tests::scattered_letters_do_not_match` (fails without the
  gate).
- **F763 G280 fixed: `accept-launcher-files.sh` step 4 looks for any file row.** With
  `files_backend = "off"` it fails on a seeded file's name in any section (Top Hit, Recent,
  anywhere), on a Documents section, and on rows that do not read as JSON; `--self-test` runs
  the check over a table without a compositor (the first case, a file as the Top Hit, passed the
  old check).
- **F764 Acceptance.** Gates: fmt, clippy (`--all-features`, `-D warnings`), `cargo test
  --workspace --features sill/debug` (1108 passed, 0 failed, 51 ignored over 113 binaries),
  `check-boundary.sh`, `cargo deny check licenses`: all exit 0. On the nested cosmic-comp
  (wayland-1 pinged, reused): `accept-launcher-v2.sh` 9 PASS, exit 0 (10 warm openings p50
  132.7, p95 229.8 ms, under a load average near 100 from other builds);
  `accept-launcher-files.sh` 12 PASS, exit 0 (step 4: "no file row anywhere (Top Hit included),
  no Documents section"). Its first run under that load failed step 1 twice (the probe's
  `M9-REP` answered `partial 0`, and `launcher-rows` had no reply within 2 s), and passed
  unchanged on the rerun a minute later: timing, not these fixes. `accept-launcher-v2-real-input.sh`
  on headless KWin: steps 1-5 PASS, exit 0; step 4 now reads Down 0→8, Down 8→16, **Right
  16→17**, Left 17→16, Up 16→8, and Enter copies cell 8. Q340 and Q341 still OPEN (quire's).

## M9 close: the emoji grid on vello_hybrid (2026-09-27)

`examples/emoji_paint_bench.rs` now times the real grid (300 `.ds-emoji-text` cells, 56 px
cells, `--fs-emoji-cell`, 8 columns, 180 frames of a 60 Hz scroll) through quire's Q330
harness backend (`HarnessConfig::with_backend`, `Harness::paint_timed`: the whole frame, GPU
included), at 100/150/200 %. Release build, this machine, while other builds ran:

| Renderer | 100 % p50 / p95 / max | 150 % | 200 % |
| --- | --- | --- | --- |
| vello_cpu | 7.5 / 10.6 / 22.8 | 6.9 / 10.8 / 23.5 | 8.6 / 13.6 / 22.3 |
| hybrid, RTX 5070 Ti | 4.7 / 6.7 / 44.1 | 5.9 / 6.7 / 9.6 | 6.4 / 8.3 / 9.8 |
| hybrid, AMD iGPU | 5.4 / 8.2 / 11.2 | 6.0 / 7.7 / 9.2 | 6.4 / 8.0 / 10.3 |

- **F765** Every renderer's p95 is inside a 60 Hz frame (16.7 ms) at every scale, including the
  user's 200 %; the 44 ms max on the discrete GPU at 100 % is its first frame (pipelines built
  on first use), not the scroll. The grid needs no virtualisation (quire: off-screen cells cost
  about 4.5 µs each). The M9 open risk (F657) is closed.

## Real faces under shell-host (2026-09-27)

Against master b454262, the first build whose daemon registers quire's faces with shell-host
(`sill_surfaces::host_config()`, shell-host F68, Q342). Until then every nested capture drew
Noto Sans wherever sill asked for Inter, Inter Display or Space Mono. Numbers from F780, Q360,
G290.

- **F780 How the scripts ran.** Every `dev/accept-*.sh` ran on the nested cosmic-comp
  (wayland-1 pinged, reused, never restarted), one at a time. Each ran under a wrapper that
  gives the whole script:
  - its own XDG_RUNTIME_DIR, holding only a symlink to the wayland-1 socket;
  - a scratch HOME and XDG_CONFIG/DATA/STATE dirs;
  - a scratch XDG_CACHE_HOME kept across runs, so the wallpaper's resize cache is warm;
  - ONE private dbus-daemon (`dev/private-bus.conf`) as both the session and the system bus;
  - no X display.

  Scripts that already isolate themselves still do so inside the wrapper.
  `accept-launcher-v2-real-input.sh` ran on its own headless KWin. The other `*-real-input.sh`
  scripts were not run (by instruction). The bar's script needs a daemon already up, so the
  wrapper started one (scratch config, the test wallpaper) and the script's last step quit it.

  The isolation withholds some services, and the new captures show it:
  - no PipeWire, so the volume reads "No output device";
  - no NetworkManager, BlueZ or power-profiles-daemon, so the Wi-Fi and Bluetooth tiles read
    "Unavailable" and there is no Power Mode row;
  - no logind, so the power menu's dry run offers nothing;
  - no Tracker on the session bus.
- **F781 Results, before (b454262) and after the fixes below.** Each cell is steps
  PASS/FAIL, then the exit code:

  | script | before | after | note |
  |---|---|---|---|
  | bar (daemon started first) | 9/0, 0 | 9/0, 0 | |
  | bar-ui (headless KWin) | 4/0, 0 | 4/0, 0 | JSON verdicts |
  | calendar | 3/1, 1 | 3/1, 1; 4/0 on a rerun | the edit step's 2 s budget, G292 |
  | clipboard | 6/0, 0 | 6/0, 0 | |
  | control-center | 4/2, 1 | 4/2, 1 | Wi-Fi pane and module item: no NM (F780) |
  | dock | 6/0, 0 | 6/0, 0 | |
  | hot-corners | 5/0, 0 | 5/0, 0 | |
  | icons | 7/0, 0 | 7/0, 0 | captures unchanged (no text) |
  | launcher-files | 11/1, 1 | 11/1, 1 | step 5 detect: no Tracker on the private bus |
  | launcher | 3/0 + SKIP, 2; 2/1 on two later runs | 3/0 + SKIP, 2 | F785; the SKIP is the wtype shortcut (M4 perf-ime) |
  | launcher-ui | 5/1, 1 | 5/1, 1 | the 100 ms budget, G292 |
  | launcher-v2 | 9/0, 0 | 9/0, 0 | |
  | launcher-v2-real-input (KWin) | 5/0, 0 | 5/0, 0 | |
  | lock | 7/1, 1 | 8/0, 0 | the first run read a half-written capture: timing |
  | notification-center | 6/0, 0 | 6/0, 0 | |
  | notifications-bus | 7/0, 0 | 7/0, 0 | |
  | notifications | 7/0, 0 | 7/0, 0; 6/1 once (shown in 409 ms, budget 300) | G292 |
  | osd | 5/0, 0 | 5/0, 0 | |
  | polkit | 7/1, 1 | 8/0, 0 | the first run captured mid-shake: timing |
  | power-menu | 4/2, 1 | 4/2, 1 | arrows and Enter: no logind offers (F780) |
  | screenshot | 5/0, 0 | 5/0, 0 | |
  | screenshot-drag (KWin) | 3/0, 0 | 3/0, 0 | |
  | switcher | 4/3, 1 | 7/0, 0 | the first run under a load average near 40: timing |
  | wallpaper | 4/0, 0 | 4/0, 0 | |
  | widgets | 6/0, 0 | 6/0, 0 | but see G290 |

  No failure was a width or font assumption that moved. Each one did one of three things:
  - passed on a rerun at lower load;
  - failed the same way with the faces switched off (F782);
  - needed a service the isolation withholds.
- **F782 Faces on against faces off.** To tell font cost from host load, the same scripts also
  ran on a build with `fonts: faces()` commented out (not committed).
  - launcher-ui's warm openings took 242-421 ms without the faces and 251-498 ms with them.
    The budget is 100 ms; FINDINGS "M4" had 85-145 on this debug build at a lighter load. The
    faces cost nothing measurable.
  - The calendar edit showed in 1585 ms without the faces. With them it took 1901, 2332, 2501
    and 4089 ms over four runs, at load averages from 6 to 44. That is noise around the 2 s
    budget, not a trend.
- **F783 What the faces changed on screen.** Every capture in `dev/out/` was read before and
  after.
  - Inter is wider than Noto Sans at the same size, so the bar's clock grows about 15 px
    ("Sun 27 Sep 01:42"). `.sill-bar-clock`'s `tabular-nums` does reach parley (Blitz maps
    `font-variant-numeric` to `tnum`), so the clock will not jump with the minutes.
  - The power menu's `←` `→` keycaps now draw their arrows; Noto's fallback drew dashes.
  - Launcher v2's trailing time and chord no longer run together: "01:19⌘R" is now
    "02:12 ⌘R".
  - The data-face labels (the control center's THEME, ACCENT and MOTION, and the calendar's
    month) no longer fall back to the system monospace face.
  - The bar-ui demo's Now Playing line fades one word earlier ("Miles Davis — Kind"); the fade
    is doing its job.
  - No label wraps that did not wrap before, nothing new is clipped, and no baseline moved.
  - The dock's context menu wraps "New terminal — COSMIC Terminal" to two lines at quire's
    220 px Context width, as `dock/menu_geometry.rs` expects. The old capture showed no menu
    at all.
  - The z-index hint from quire's "A part's leave is not the row's enter" was checked against
    sill's positioned boxes: no press failed in any run.
- **F784 Up Next's title no longer fades when it fits.** `.ds-truncate`'s fade always covers
  the column's last 1.5em. `.sill-un-text` was sized to its words, so the fade covered the
  title's own last letters: "Design revie(w)" faded beside 150 px of empty tile, before and
  after the faces. The words column now takes the rest of the row (`flex:1 1 0`, as the control
  center's Now Playing already did). "Design review" now draws whole
  (`dev/out/calendar-cosmic.png`).
- **F785 `accept-launcher.sh` step 2 counted a frame of the opening as idle.** `before` was
  read right after the capture and before the close. So a frame of the entrance still in flight
  (a loaded host, a debug build) failed the step (17 → 18, 9 → 10). `before` is now read 0.5 s
  after the close.
- **F786 Goldens and gates.** Nothing under `tests/` is a pixel or text snapshot of a nested
  surface, and no test changed: the in-process harness already used Inter. All five gates exit
  0:
  - `cargo fmt --all --check`;
  - clippy (`--all-features`, `-D warnings`);
  - `cargo test --workspace --features sill/debug`: 1109 passed, 0 failed, 51 ignored over 114
    binaries;
  - `check-boundary.sh`;
  - `cargo deny check licenses`.

  Two earlier test runs failed for reasons outside this change:
  - one hit the user's /tmp quota (rustdoc: "Disk quota exceeded");
  - in one, `launcher_card` failed once while quire's master moved underneath, and passed on
    the rebuild.
- **F787 Captures.** All captures were retaken except two:
  - `power-menu-cosmic-restart.png` needs the Enter step, which needs logind's offers;
  - the icon strips came out identical.

  Under F780's isolation:
  - the control center's Wi-Fi and Bluetooth tiles read Unavailable;
  - the power menu's buttons are all disabled;
  - `control-center-wifi-cosmic.png` shows the grid, not the Wi-Fi list. The old capture
    listed the neighbours' real network names.

  Some scripts set their own XDG_CACHE_HOME: polkit, screenshot, and the calendar's widget
  shot. They resize the wallpaper cold and shoot before it lands, so their backdrop is plain.
  Two captures are new: `polkit-typed-cosmic.png` (Q360) and `widgets-center-all-surface.png`
  (G290).

### Asks and gaps

- **Q360 (quire, OPEN) A typed `Secret` field draws its bullets from the middle and its caret
  before them.** After typing "wrong-guess" into the polkit prompt, the eleven bullets start
  about 90 px into the field, about the plain text's own width, and the caret stands at their
  left (`dev/out/polkit-typed-cosmic.png`, the `sill debug capture` of the prompt). It looks
  the same with the faces switched off (F782), so the faces are not the cause. The cause is
  `TextInput{Secret}` over Blitz's password input. The lock screen's field is the same
  component; no script types into it.
- **Q361 (quire, OPEN) The compact MonthGrid's today disc barely holds two Inter digits.** On
  the desktop's Small Calendar, "27" touches the disc's edge, and "26" sits 4 px from it
  (`dev/out/widgets-desktop-cosmic.png`, top right). Under Noto the digits had room.
- **G290 (shell-host, OPEN) The on-screen frame puts two widgets' contents in the first tile.**
  With `widgets.center` = all five, the nested screen shows Now Playing and World Clock drawn
  over the Calendar tile, and tiles four and five empty. `sill debug capture` of the same
  surface is right (compare `dev/out/widgets-center-all-cosmic.png` with
  `dev/out/widgets-center-all-surface.png`).
  - The Noto capture of 3a81779 has it too, so the faces are not the cause.
    `accept-widgets.sh` counts tile frames on the capture, so it passes.
  - The tiles' bodies are `position:relative` with a `z-index` token (quire's WidgetFrame).
    The presented frame misplaces these raised layers while the capture places them right.
    That points at shell-host's present path.
- **G291 (sill, closed below "G291 Up Next fit") Up Next's third event is clipped at the
  tile's foot.** `up_next_max` 3 in a 164 px tile cuts "Tomorrow 09:00" in half
  (`dev/out/calendar-cosmic.png`). It did so before and after the faces.
- **G292 (sill, OPEN) Timing budgets flake on a loaded debug build.** None of these tracks the
  faces (F782):
  - launcher-ui's 100 ms budget: 251-498 ms here, faces on or off;
  - the calendar edit's 2 s budget: 1.6-4.1 s;
  - the banner's 300 ms budget: 409 ms once.

## G130 root cause (2026-09-27)

`notification_center_render`'s four click tests fail under load (10/10 runs failed with 32
`yes` spinning; 11/11 pass on an idle machine). The cause is not the waits and not sill's
surface: it is a Blitz style bug that quire's wall-clock presence timer exposes.

- **G295 Blitz keeps a cancelled CSS animation's last value on the element, for good.** When
  an element's `animation-name` goes away before its animation has ended, the restyle that
  cancels it cascades the animation's value at that moment, and nothing restyles the element
  again: stylo sets `ElementAnimationSet::dirty` for that, but Blitz's `resolve_stylist` never
  reads it, and `clear_canceled_animations` plus `sets.retain(non-empty)` then drop the set, so
  the "animating nodes are dirty" pass never sees the node again. The stale transform is
  painted and hit-tested; `get_client_bounding_rect` (layout, no transforms) reads the resting
  box, so `Harness::rect` and `centre` look fine. Deterministic repro, no wall clock:
  `tests/cancelled_animation.rs` (a box sliding in from `translateX(400px)` over 1 s, its class
  removed 20 ms in: two seconds later it still paints and hit-tests 392 px to the right;
  removed after its end, it rests). Ignored until the Blitz pin carries a fix (restyle the
  nodes of dirty sets before dropping them, as Servo does). The fix belongs in quire's Blitz
  pin, not sill.
- **How the center hits it.** quire's `Panel` holds `data-presence=entering` (the `panel-in`
  animation, `translateX(100% + 16px)` → none) until a `use_motion_timer(PanelIn)` fires on the
  wall clock, while the animation runs on the harness clock (`Harness::advance` moves CSS time
  at most the step asked for). Under load the first frame after `link.open` takes 700 ms-3 s of
  wall time (probe: the first `advance(20)` returned after 769 ms at load average 125), so the
  timer fires and `presence` flips to `present` when the animation is ~2 % in. The panel is
  then frozen about 392 px right, past the viewport: the probe found no x in 888..1280 at the
  title's row that hit `.ds-panel`, from the first frame on and 400 ms later, with
  `presence=present`, `is_animating()=false` and `rect(.ds-panel).x = 888` throughout. Every
  click at a `centre()` inside the panel lands on `.sill-nc-catcher`, which closes the center:
  hence "the row count never reaches 4", no Clear commands, and "a press inside closes it".
  A "settled" wait (presence present, not animating, rect stable) would pass at once and not
  help: all three already hold while the panel is displaced.
- **Not only tests.** In a real session both clocks are the wall clock, so the entrance is
  cancelled only as far along as the first style resolve lagged the render; an e-out curve
  hides a small lag, but a stalled first frame leaves the center displaced and its clicks
  falling through until the next open. The OSD card runs the same `use_shown_phase`; other
  timer-ended entrances were not checked.

## G291 Up Next fit (2026-09-27)

Closed. The tile's body is a fixed number of pixels (quire's `WidgetFrame`: `--widget-cell`
164 default, `--widget-gap` 16, 12 px card padding), not something the widget's own settings
can grow, so asking `calendar.up_next_max` (default 3) for more rows than the body holds drew
the last one half off the card's foot, cut by `.ds-widget{ overflow:hidden }`.

- **Measured, not assumed.** A throwaway `ds_native::Harness` render of `UpNextWidget` at
  the default tokens with three qualifying events gave the real rects: the frame is 344 x 164
  (Medium; UpNext is never drawn Small or Large, `widgets::size_of`), the body 318 x 117.5 at
  (13, 33.5), each event row 39.53 px tall with a 6 px (`--s-6`) gap between. Two rows
  (2 x 39.53 + 6 = 85.06) fit inside the 116.5 px available under the body; three (130.59)
  do not — the third row's rect bottom (166.08) sat past the card's own bottom (164), which is
  the clip. A Small tile shares Medium's height (only a Medium's width differs, `.ds-widget`'s
  CSS has no `[data-size=medium]` height rule), so it holds the same two; a Large tile, two
  cells and a gap tall (344), holds six by the same arithmetic. These numbers are baked into
  `events_capacity` (`crates/sill-surfaces/src/widgets/up_next.rs`) rather than measured again
  on every render, matching quire's own `MonthDensity::resolve` precedent (deterministic from
  `WidgetSize`, not the configurable `widgets.desktop_cell_px`, and not a live text
  measurement).
- **Option (a) chosen.** design/23-WIDGETS.md section 5 (the Calendar widget's queued restyle)
  already answers "how many events does a small or medium widget show": Small draws the date
  face and *one* next event ("or 'No more events today'"), Medium the date face and the *next
  two* — fewer whole events at the smaller sizes, never a clipped one. That is a different
  widget (the month-plus-hero-date card, not Up Next's list), so its exact counts do not carry
  over, but its principle does, and it is the same principle the task asked to follow. Option
  (b), retuning `--s-6` and the row's own type sizes so three fit at 164 px, was rejected: quire
  owns those tokens and the card's height is fixed by `--widget-cell` (a setting), so shrinking
  the row to fit three is a target chosen to match a count, not a size, and would still break
  again the next time someone raises `up_next_max` or lowers `desktop_cell_px`.
- **The fix.** `events_capacity(size: WidgetSize) -> usize` (Small/Medium: 2, Large: 6);
  `UpNextWidget` now asks for `min(calendar.up_next_max, events_capacity(size_of(UpNext,
  host)))`, so `up_next_max` is a ceiling on top of the size's own room, never a promise past
  it. At the defaults this drops the display from 3 requested to 2 shown; the third (soonest
  excluded) event simply does not draw, whole events over a clipped one.
- **Tests.** A table test (`a_size_holds_as_many_rows_as_its_cell_measured_out`) pins the three
  `events_capacity` cases. A new `ds_native::Harness` test in
  `crates/sill-surfaces/tests/widgets_render.rs`
  (`up_next_never_clips_a_row_past_the_tiles_body`) renders `Widget { kind: UpNext }` in both
  hosts with three qualifying events at the default settings and asserts every `.sill-un-row`'s
  rect stays inside `#widget-up-next .ds-widget-body`'s rect; it passes in both. The existing
  `up_next_lists_the_next_events_in_order_with_their_colour` (`calendar_render.rs`) already only
  ever gave two qualifying events, so it needed no change.
- **`dev/accept-calendar.sh` updated.** Its fixture asks three calendars for events (work in an
  hour, team's Yoga in three, personal's Dentist tomorrow at 09:00) and used to assert all
  three colour bars drew, which was really asserting the clip: the third bar's top sliver was
  still detectable by the pixel probe even half-cut. Step 1 now expects only `work team` (the
  two soonest); the comment block explains why personal's bar does not draw. Steps 3-5 needed
  no change: after work's event is cancelled only two events remain, which is exactly what
  capacity 2 holds, so the existing `team personal` assertions already matched the fixed
  behaviour.
- **Verified.** All five gates: `cargo fmt --all --check`; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`; `cargo test --workspace --features
  sill/debug` (1111 passed, 0 failed, 97 binaries); `scripts/check-boundary.sh`; `cargo deny
  check licenses`. `dev/accept-calendar.sh` on the nested compositor (wayland-1, pinged and
  reused): 4/0 steps, exit 0 (`capture: BARS 1 5 ; DISC 276 225 24 28`, work then team, no
  personal). `dev/out/calendar-cosmic.png` re-captured and read: Up Next shows "Design review
  04:05 – 04:35" and "Yoga 06:05 – 07:05" in full, nothing cut off, nothing beyond the tile.

## Launcher details (Q297) (2026-09-27)

Adopting quire's `ds::detail` (design/26-DETAILS.md, wave D0; quire CONSUMING "Details") in the
launcher, against sill master 3f7e50b and quire master. Numbers from F800, Q370. Arc heuristic
(quire design/09): **H6, power to the people**: every launcher moment now goes through quire's
primitives, which honour the person's motion level (Reduced: final state at once; Calm: no
overshoot) in one place instead of per surface, and a spring is spent only on the person's own
contact (design/26 R5, R7).

- **F800 The states and their tables.** `preview/moment.rs`: `PaneState::{Hidden,
  Showing(PreviewKind), Pending, Failed}` with an exhaustive `Detailed` table (no `_` arm):
  Hidden to Showing or Failed is **Appear**; another kind while shown, or Failed to Showing, is
  **Preview** (the selection moved; the entrance never replays; the same kind is Rest, R2);
  anything to Pending is **Pending**; Pending to Showing is **Change** (a load landing is content
  arriving, not a success to settle, R12); Pending to Failed is **Failure**; to Hidden is
  **Dismiss**. First frames: Hidden Rest, Showing/Failed Appear, Pending Pending.
  `list_detail.rs`: `ListState::{Closed, Waiting(opening), Empty(opening), Rows { opening,
  answer: Generation, expanded }}`: the first answer's rows after an opening are **Appear**
  (design/26 §8 item 6: only the first set staggers), a new answer is **Change** (in place, no
  stagger: typing is frequent, R1/R12), Show More and Show Less (`expanded` differs, same
  answer) are **Change**, "Nothing matches" and back are **Change** (G43's fade), a new opening
  plays its own first frame, closing is **Dismiss**. Both tables are data tests
  (`moment_table`, `first_table`); `pane_state` and `load_of` have table tests too.
- **F801 Where `use_detail` is called.** `LauncherPane` calls `use_pane_detail` (`FirstShow::
  Animate`: the pane mounts when the person shows it), from the shown/hidden prop and how far
  its content has loaded (`content::Load::{Ready, Reading, Failed}`: a text head `Reading`, a
  PDF page `Loading`, their failures). The panel calls `use_list_detail` once per render. The
  frozen no-op `use_pane_moment`/`use_row_moment` and `PaneMoment` are gone.
- **F802 Touch.** `contact.rs`: the panel root's `onkeydown` and `onmousedown` record
  `Touch::from_event` and bump a signal, so the render that follows takes it (then Remote); a
  contact never lingers onto the next remote change. An answer arriving, a load landing and every
  `sill debug launcher-key` are `Touch::Remote`. The pane gets the render's touch as a prop.
- **F803 What plays today: nothing new.** The cues are computed and dropped (`_pane`,
  `_results`): quire's `PreviewPane` and `CommandPalette` take no `Cue`, `Touch`,
  `FirstShow` or `Operation` (Q370-Q372). So quire's pane still plays its own `slide-r` at
  `--t-move --e-spring` on every showing (contact or not) and `pane-out-r` on hiding; the rows
  still show with no rise; a slow text head still shows an empty media box (a slow PDF page
  shows `PdfThumb`'s own grace-then-half-strength sheet, not `use_pending`). `use_operation`
  and `use_pending` are **not** called: with no part to draw the frame they would only wake the
  panel every `--t-pending-step` for up to `PendingCap` for nothing (R3). No local stand-in was
  written (no sill CSS, no ad-hoc `rise`), per the brief.
- **F804 Lint.** `tests/launcher_details_lint.rs`: `launcher.css` is clean under
  `Profile::Details` (no `InfiniteLoop`, no `OffGrammarTiming`), and so is the rendered panel's
  markup with the pane up. Nothing in the sheet had to change.
- **F805 Idle frames.** `tests/launcher_details.rs`: open over `m9`, then Quick Look up, Down
  under it, Quick Look off: `ds_native::harness::assert_settles_to_zero_frames` passes after
  each step (no CSS animation, no `Harness::wakes()` for 500 ms).
- **F806 The reference's motions** (micro-interactions note; design/26 §5.6, confidence L,
  observed from the shipping product, not measured): Spotlight's results replace in place as
  you type and the window grows with them; rows do not stagger (our first-set rise is design/20
  §1.3's, not the reference's); the preview beside the results changes with the selection at
  once, a generic icon first and the thumbnail replacing it when ready; section "Show More"
  expands in place. That is what the tables above encode: Preview/Change in place, one Appear
  per opening.
- **F807 Gates and scripts.** fmt, clippy (`-D warnings`), `check-boundary.sh` and `cargo deny
  check licenses` exit 0; `cargo test --workspace --features sill/debug` exits 0 (1119 passed,
  0 failed, 52 ignored, 117 test binaries). The first run failed one unrelated timing test,
  `hot_corners_render::leaving_and_the_rearm_lets_it_fire_again`, at load average 142-191; it
  passes alone and in the full rerun. `dev/accept-launcher-v2.sh` on wayland-1: every step
  PASS (latency p50 263.4, p95 374.1 ms under that load; sections; PDF and image preview; pane
  levels; Show More; emoji; real caret; **step 8 idle: 189 frames before and after 5 s, so 0**;
  fixtures). `dev/accept-launcher-v2-real-input.sh` on headless KWin: all five steps PASS.

### Asks (quire)

- **Q370 `PreviewPane` takes the detail's cue.** Today `shown: Shown` alone drives
  `use_shown_phase`, whose entrance is `--e-spring` whoever caused it, breaking R5 for a remote
  showing (`sill debug launcher-key quick-look`, a future Quick Look request over IPC). Proposed:
  `cue: Option<Cue>` (or `touch: Touch`) so the slide-in springs only on `Touch::Contact` and
  settles at `--e-out` otherwise, and a `Preview` moment cross-fades the media at `--t-quick`
  instead of snapping.
- **Q371 `PreviewPane` draws a pending frame.** A slow text head (`PaneContent::Text` with an
  empty excerpt) is a blank box; `PdfThumb` has its own grace (`PDF_THUMB_GRACE`) outside
  `use_pending`. Proposed: `operation: Operation` (sill passes `use_operation(detail.cue())`),
  drawn with `use_pending` over the media box (a `Spinner` or `LayerGlyph` pending frame, its
  still frame at the cap, the words "Loading…" for R8), and `PdfThumb`'s grace moved onto the
  same primitive.
- **Q372 `CommandPalette` plays the rows' first show from a cue.** The rows are the palette's,
  so sill cannot wrap them in `ds::detail::Reveal`. Proposed: `reveal: Option<Cue>` (or
  `first: FirstShow` per result set) so an Appear cue plays `rise` with `--stagger` (cap 12) on
  that set only, and a Change replays nothing. `Reveal` itself only plays at mount; the palette
  outlives result sets, so it needs a keyed replay or a cue-driven variant.
- **Q373 (design/26) Show More / Show Less has no row.** §5.6 lists open, results, calculator,
  no results and selection; §5.4's group expand (Reveal downward, `heal` to collapse) is the
  nearest. sill calls it Change until the grammar decides; the palette would then need the same
  cue for the section's new rows.

## Virtual clock (Q380) (2026-09-27)

quire master 0394c5d5 gives `ds_native::Harness` a virtual clock (`HarnessConfig::with_clock(
Clock::Virtual)`): `advance` steps through every ds timer in order and resolves CSS at the same
instant, never waiting on the wall clock. sill's Harness tests move onto it; the ones that
flaked under load (G130, G295's shape) are now exact.

- **Production time goes through `ds::time`.** `FrameClock::now` reads `ds::time::now` (the wall
  clock in the daemon, the harness's clock in a test), and `FrameClock::started` replaces
  `FrameClock::new(Instant::now())` in the dock, OSD, switcher and banner column; the launcher's
  open/toggle instant and its first-frame latency line read `ds::time::now`/`since`. Every
  sill-surfaces timer was already `ds::sleep`. Left alone: civil time (`SystemTime`,
  `jiff::Zoned::now`, `Timestamp::now` for "5 min ago", the lock clock, the world clock, Up Next)
  and `sill-services`' tokio timing.
- **Converted.** Every sill-surfaces Harness test outside D1's files: banners, calendar,
  cancelled animation, desktop widgets, hot corners, the launcher (actions, card, catcher, details,
  keys, lint, PDF rasters, preview, render), notification center (keys, lint, render), OSD,
  polkit prompt, power menu, screenshot thumbnail, switcher, wallpaper paint, widgets (lint,
  render) and the dock menu document. Waits bound on the harness clock (`settle_until`, or
  `harness.now()` for the helpers that report `Done`); no test reads `Instant::now()`.
  Boundaries are exact: the OSD hold (`hold - 1ms` present, `hold` leaving), the banner hold
  (`FRAME_SLACK + hold`), the screenshot hold, the switcher's show delay, the hot corner's dwell,
  and a new test for the re-arm (`rearm - 1ms` stays spent, `rearm` fires again). Load run: 32
  `yes`, each file 10 times, 340 runs, 0 failures.
- **The launcher's fake search runner runs on the harness thread.** It was a `std::thread`, and
  off-thread work never lands on the virtual clock. `launcher_support::Runner` keeps the thread
  for `launcher_demo` (`on_thread`) and runs as a task the harness root spawns (`run`).
- **One harness per thread at a time.** Each installs its virtual clock on the thread until it
  drops, and the guards restore in LIFO order; `launcher_card` measured a short and a long card
  from two live harnesses and now drops the first before opening the second.
- **G310 The actions menu hangs from the last row rect the palette reported.** quire's palette
  reports the selected row's rect a `FRAME_SLACK` after the selection (`laid_out_rect`); a
  Ctrl+K inside that window hangs the menu under the previous row. The wall-clock test passed on
  the slack of real time; it now advances `FRAME_SLACK` after Down. A user is never that fast;
  not fixed.
- **Latent: a wait that never matched.** `shift_tab_with_nothing_typed_reaches_emoji_alone`
  waited for three `.ds-menu-item`s, which an emoji grid never has; the old two-second wall loop
  timed out silently and the test passed on what the grid drew meanwhile. `settle_until` panics,
  which found it; it waits for three `.ds-emoji-cell`s now.
- **G311 On the virtual clock a feed's wake runs at the next stop, which is the advance's end.**
  A test's `watch` send or link ask between advances wakes the surface's task, but `advance(d)`
  with no timer due runs the woken task only at `d`, so the "asked at" instant slides to the end
  of the next advance (the switcher's chord then showed at 260 + 150 ms, not 150). The support
  helpers (`ask`, `show`, `shot`, `cleared`) follow each send with `advance(Duration::ZERO)`.
  quire could run what is woken at the current instant before looking for the next timer.
- **G312 `ds::focus_soon` lands at once or a `FRAME_SLACK` later, varying run to run.** On the
  virtual clock the switcher's key target had the focus 5 ms after the ask in 9 of 20 runs and
  only after 34 ms in the rest (the host write found the document busy and retried). Something
  besides the clock still orders the harness's tasks (task or hash order); the test asserts
  the focus after `FRAME_SLACK`, not the boundary.
- **After D1 merged.** The bar's `tray_menu_tests`, the control center's `cc_support/live.rs`
  (so `control_center_render`), D1's `status_glyphs` and `bar_low_nudge`, and `widgets_render`'s
  own harness moved too; `bar/session.rs` takes `FrameClock::started`. Load run: 32 `yes`, 10
  each, 50 runs, 0 failures.
- **Left on Wall.** No sill-surfaces Harness test. `examples/emoji_paint_bench.rs` is a paint
  benchmark, not a timing test. G295's midway test still fails on the current Blitz pin and
  stays `#[ignore]`d.

## D1 status glyphs (2026-09-27)

Adopting quire's layered status glyphs (quire CONSUMING "Status glyphs", design/26 wave D1,
design/04 section 51) in the bar and the control center, against sill master afc8fbc merged
with gate 198a0d0 (Q297) and quire master 0d51b07f. Numbers from F820, Q390, G300.

**Where it stopped.** Every place these glyphs would go takes an `Icon`, not an element:
`IconButton { icon: IconSource }` (the bar's status items), `ModuleTile { glyph: Icon }` (the
Wi-Fi and Bluetooth tiles), `ModulePanel { glyph: Option<Icon> }` (the Battery module's header)
and `LevelControl`'s `LevelGlyph` (the Sound module). No quire component puts a `StatusGlyph`
in a status item's box with its pill, ink and label, so drawing one there means raw markup or a
local copy of `.ds-icon-button[data-variant=status]`, which the brief forbids (Rule 3). The
lane did everything under the rendering and stopped there: Q390-Q392. The bar still draws the
old `Icon` buckets, and still shows `WifiOff` while connecting, until Q390 lands.

- **F820 Join and connect stamps (the bug's service half).** `sill-services/src/attempt.rs`:
  `OpStamp(u32)`, `Attempt::{Quiet, Running(OpStamp), Failed(OpStamp)}`, `Attempts { now,
  minted }` and `Attempts::next(AttemptStep::{Start, Fail, Land, Hold})`, a pure reducer.
  - A start while one runs keeps its stamp, so the pending loop keeps its grace and cap (R4).
  - A start otherwise mints a new stamp, and so does every failure, so each failure shakes
    once (R6).
  - `Gone` keeps the count, so a stamp is never reused while the daemon runs.
  - `Network.join` is stepped by the link: entering `Connecting` is Start, `Connecting` to
    `Disconnected` is Fail, any up link is Land, and anything else is Hold.
  - `Bluetooth.connect` is stepped by a device's connection: entering `Connecting` is Start,
    `Connecting` to `Disconnected` is Fail (`run_connect` already reverts a failed `Connect`
    this way), `Connecting` to `Connected` is Land, and the radio off or the adapter gone is
    Land.
  - Both new fields are `#[serde(default)]`. Table tests: `attempt::tests`,
    `network::model::tests::each_join_is_stamped_once_and_each_failure_anew`,
    `bluetooth::model::tests::each_connect_is_stamped_once_and_each_failure_anew`.
- **F821 The mapping.** `bar/glyph_state.rs` (pub, pure), one table test per function in
  `glyph_state_tests.rs`, covering every row of CONSUMING's table:
  - `network_glyph(&Network, Radio) -> NetworkGlyph::{Wifi(WifiState), Plain(Icon)}`.
    `Connecting` is `Joining(EventStamp(n))` with the join's stamp. A Wi-Fi link is `Joined {
    bars: WifiBars::of(Fraction(strength*10)), reach }`. Connectivity None, Portal or Limited
    gives `NoInternet`; Full or Unknown gives `Internet`. Disconnected is `Off` with the radio
    Off or Blocked, `Failed(stamp)` while the last join's failure stands, else `Idle`. Wired and
    Other keep `Ethernet` and `Link`.
  - `battery_glyph(&Battery, bar.battery_low_percent) -> Option<BatteryState>`. Level is
    `Fraction(p*10)`. Charging gives `Charging`; Full or Idle gives `Held`; the rest give
    `Battery`. `low_at` is `LowAt(Fraction(key*10))`. `None` when absent, so the item hides.
  - `volume_glyph(&Volume) -> VolumeState`. Absent is `NoDevice`; muted or level 0 is `Muted`
    (the slash stays); the rest is `Heard(VolumeWaves::of(..))`.
  - `bluetooth_glyph_state(&Bluetooth) -> BluetoothState`, in this order: Off (no adapter or
    radio off), `Connecting(stamp)`, `Connected` (any device), `Failed(stamp)`, `On`. A failure
    beside an already connected device reads Connected, so the words stay true; that failure
    does not shake (G301).
- **F822 G11, the low-battery nudge (built, on today's item).** `bar/low_battery.rs`:
  `LowWatch::{Clear, Low}`, a reducer over the item's `BatteryState`.
  - Clear becomes Low at or under `low_at` on battery.
  - Low stays Low while on battery, even when the charge wobbles back over the threshold. Only
    Charging or Held (or no battery) re-arms it, so the item nudges once per discharge, never
    in a loop (R6).
  - Its `Detailed` table: Clear to Low is **Attention**, everything else is Rest, and the first
    frame is Rest (bar chrome, R1: a session that starts low has crossed nothing).
  - `use_low_nudge` feeds `use_detail(.., FirstShow::Still, Touch::Remote)` to quire's
    `use_nudge`. `StatusIcon` puts the `PulseKey`'s class and `data-pulse` on a new wrapper,
    `span.sill-bar-status` (display only, no token), around the `IconButton`, as CONSUMING says
    ("put `.attrs()` on an HTML wrapper"). The slot still measures the button.
  - Unplugging while already low nudges once again: a new discharge crossing. When Q390 lands,
    the wrapper moves onto the glyph, and the fill's `--battery-low` (G10) carries the same fact
    at rest (R8).
- **F823 Settings.** `bar.battery_low_percent: Percent`, default 20, range 5..=50, Advanced,
  section Bar. It is in `AWAITING_ROWS`. No Bluetooth key is needed: the bar's Bluetooth item
  already exists behind `control_center.menu_bar_bluetooth` (`InMenuBar::{Show, Hide}`,
  default `Hide`, a design/22 row since M5), which is the key design/26 5.1.2 names. Neither key
  belongs on a Basic page: §5 lists no `bar.*` or `control_center.*` key outside Advanced, and
  the reference has no low-battery threshold setting. Proposed row for design/22 §3.4:

  | Key | Type | Default | Range / Alt | Source | Status |
  | --- | --- | --- | --- | --- | --- |
  | `bar.battery_low_percent` | `Percent` | `20` | `5..=50`; at or under this charge, on battery, the bar's battery fill turns `--battery-low` and the item nudges once per discharge | `26-DETAILS.md` 5.1.3 G10, G11; sill FINDINGS F822 | proposed (D1, 2026-09-27) |
- **F824 Harness tests.** `tests/status_glyphs.rs`: the fake services, mapped by
  `glyph_state`, drive quire's glyphs on a Blitz document, and every change ends in
  `assert_settles_to_zero_frames`.
  - Wi-Fi: `Connecting` searches (one layer lit, no slash), a join fills to two bars, and a
    portal grows the badge. A failed join shakes once and the same stamp does not shake again;
    the radio off slashes.
  - Battery: 18 % turns the fill low, charging shows the bolt and is never low, full shows the
    plug, and absent hides it.
  - Volume: 40 % shows two waves, 0 shows the slash with no waves, and 90 % shows three.
  - Bluetooth: connecting breathes, then connected grows the dots.

  `tests/bar_low_nudge.rs`: the real `Bar` in a Harness. `.sill-bar-status.a-nudge-up`
  appears on crossing into 20 %, not for 15 %, a wobble to 21 % or back to 19 %, nor when
  plugged in, and again once when unplugged low. Each step ends at 0 frames.
- **F825 The reference's motions** (the micro-interactions note; design/26 5.1, confidence as
  there): Wi-Fi's bars cycle while it joins and fill on joining (M/H); a distinct no-internet
  glyph (H); the battery outline fills by level and its bolt or plug shows the source (M), and
  it turns red near a fifth (M); a notification at low battery (L). The glyphs encode these;
  the bar shows none of them until Q390.

### Gaps (sill)

- **G300 The bar still says "Wi-Fi off" while connecting.** `network_icon` is unchanged. There
  is no `Icon` for joining, and the fix is `WifiState::Joining` through Q390. The mapping and
  the stamp are ready (F820, F821).
- **G301 A Bluetooth failure beside a connected device does not shake.** The glyph has one
  state; Connected wins so the words stay true (F821). Per-device failure belongs to the
  devices pane's rows (design/26 5.2.3, G20).
- **G302 `Link::Connecting` may rarely be seen.** The network service reads NetworkManager's
  `PrimaryConnection`, which usually names an activated connection. An activation in progress
  is `ActivatingConnection`. The Joining state and its stamp only appear when the primary
  connection itself is activating. Reading `ActivatingConnection` needs a check on a real
  NetworkManager, which this lane may not touch; it is queued for the manual checks.

### Asks (quire)

- **Q390 `IconButton { Status }` holds a status glyph.** Proposed: `IconSource::Status(
  StatusState)`, or a `glyph: Option<Element>` slot, drawn in the same `--bar-status-glyph`
  box with the same ink, pill and `aria-label`, so the bar's Wi-Fi, battery, Bluetooth and
  volume items can pass sill's `StatusState`. Include a pulse slot, or say that sill's wrapper
  (F822) should carry G11's nudge. Without it, the bar keeps `Icon` buckets and G300 stands.
- **Q391 The control center's glyph slots take a status glyph.** Proposed:
  - `ModulePanel` gets the same `IconSource::Status` (or an element slot) for the Battery
    header, with `first: FirstShow::Animate` so `BatteryGlyph` sweeps in when the center opens.
  - `ModuleTile`'s disc gets the same for Wi-Fi and Bluetooth, which would also carry G14's
    fill on enable.
  - Until then, sill cannot pass `FirstShow::Animate` anywhere (item 6 of the brief).
- **Q392 The Sound module's `LevelControl` and `VolumeState`.** `LevelControl` takes a
  `LevelGlyph`. `VolumeGlyph` is `LevelGlyph`'s body, so the Sound module already draws the
  same waves and slash; say whether `LevelControl` should take a `VolumeState` for a single
  source. sill's module maps level 0 to `VolumeX` today, and `volume_glyph` is ready.
- **F826 Gates and scripts.**
  - `~/sill-wt/sill-gate.sh` with `GATE_DIR` set to this worktree: **SILL GATE GREEN**. fmt,
    clippy `-D warnings`, boundary and deny passed. Tests: 1135 passed, 1 failed, 52 ignored.
    The one failure was `launcher_card`, a load-flaky Harness file (load average 50-250 during
    the run); it passed alone on the gate's retry.
  - How the scripts ran: on the nested wayland-1, which was pinged first, reused and never
    stopped. They ran under F780's isolation: a scratch XDG_RUNTIME_DIR holding only the socket
    symlink, a scratch HOME and XDG dirs, and one private dbus-daemon as both buses.
  - `dev/accept-bar.sh` (daemon started first): 9 PASS, exit 0. Idle was 6 frames before and
    6 after, so 0.
  - `dev/accept-control-center.sh`: 4 PASS, 2 FAIL, exit 1. `step_wifi_pane` and
    `step_module_item` fail because there is no NetworkManager under the isolation, the same
    as F781's baseline. Idle: 0 frames.
  - Captures: `dev/out/bar-cosmic.png`, `bar-cosmic-menu.png`, `control-center-cosmic.png`
    and `control-center-wifi-cosmic.png`. Unchanged apart from the time: the wrapper adds no
    size, and the glyphs are still the `Icon` buckets. The battery item is absent because
    there is no UPower under the isolation, and the volume shows `VolumeX` because there is no
    PipeWire.
