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
