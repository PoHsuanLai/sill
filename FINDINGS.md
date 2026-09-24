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

## M1 bar-ui (2026-09-24)

- **F20 One popup, anchored by a measured rect.** Every bar menu is one `xdg_popup` opened with
  `bar::popup::menu_popup`'s config and `PopupAnchor::Rect`, not `Element`: ds `Button` and
  `IconButton` render no `id`, and a hover switch (`SurfaceOp::Reposition`) needs the new
  title's rect in any case. The rect is the button's own, from its `mounted` handle through
  `ds::use_rect`, and travels with each title event to the session. The popup is a document of
  its own (`menu_popup.rs`) that follows a per-popup `watch` of its view: a new view swaps the
  contents and repositions, `None` makes it close itself (a `SurfaceHandle` closes only its own
  surface). It reports picks, dismissals and its own drop on an `mpsc` the bar drains into the
  tracking machine (`plan::popup_input`).
- **F21 The ds margin is part of the popup.** A ds floating surface is clamped 8 px inside its
  bounds (`ds::place`), so the popup is the menu plus 8 px on every side and 16 px more right
  and below for the drop shadow; the anchor rect folds the margin in and `PopupConfig::offset`
  carries design/13 §13.3.2's (-8, +6) / (+8, +6) (G3, now closed upstream). The size is an
  estimate from the entries (G6); the spare area is transparent.
- **F22 What the machine drives, and what the ds Menu keeps.** The machine gets presses,
  releases and pointer entries on titles from the bar, and picks, Escape/outside-margin closes
  and drops from the popup; `Open`, `Switch`, `Close` and `Pick` become show, swap, hide and run.
  `Highlight`, submenus, the safe triangle and `Adjacent` are not wired: the ds `Menu` draws its
  own selection from the pointer and keys and exposes neither item hover nor item release (Q8),
  and bar menus have no submenus. Press-drag-release onto an item therefore needs a click on the
  item when the compositor keeps the implicit grab on the bar. A press on the bar outside every
  title is an outside press (as on macOS); a real outside press ends the grab and the popup's
  drop reports it.
- **F23 Status lines are menu headers.** ds `MenuEntry` has no disabled or informational item
  (its TODO O-1), so network, battery and clock menus show their lines as headers and end with a
  Settings item (`LaunchCommand::Launch` of `com.system76.CosmicSettings`). The volume menu is a
  ds `Popover` with a `SectionHeader{Field}`, a `Slider` and a mute `Toggle`, since a `Menu`
  cannot hold a slider.
- **F24 The bar's tint.** `Ds` paints frame layers (the A/B cross-fade) only for
  `Material::Window` (Q6). The bar paints its own layer of `--f-solid` at `--m-tint-alpha` over
  the Bar material, with `background-color` and `color` transitions over `--t-scene`, so a
  workspace switch cross-fades the hue (design/21 §5 through spike S4) but the layer is the
  Space's first stop, not its gradient.
- **F25 Keys that do not reach quire components.** `bar.status_icon_box_px`,
  `bar.status_glyph_px` and `bar.glyph_size_policy` have no effect: `IconButton{Tool}` is 28 x 26
  with a 16 px glyph and a consumer may not style `.ds-*` (Q9). `bar.open_title_pill_height_px`
  is the title's hit height (both 24 by default): the pill is the hit box's background. The app
  name is a `Button{Quiet}`, so it is not ui 13/700. `bar.height_px`, `title_hit_height_px`,
  `title_padding_px` and `status_gap_px` are written inline from the settings.
- **F26 Not built in this wave.** The workspace pills' right-click Edit Space (the Space editor
  popup is later work); the bar menus' popup blur (`BlurState::Unavailable`, the solid tint:
  a popup's document would have to name the menu element for a blur region, and ds `Menu`
  renders no id); the open and close animations design/13 asks for (the ds `Menu` always plays
  its entrance, and the popup goes at once on close).
- **F27 `use_space_look`** returns `ds::default_look(index, spaces.default_grain,
  spaces.default_card_accent)`: design/21 §4's preset for the workspace's position in its
  group (F17). With no workspaces (KWin) the bar takes preset 1.
- **F28 Acceptance** (`dev/accept-bar-ui.sh`, nested `kwin_wayland --virtual`, 1280 x 720
  logical at scale 1.5, `examples/bar_demo.rs` on fake services): a maximised probe is 720 tall
  without the bar and 688 with it (32 reserved); five idle seconds add 0 frames, 0 commits and
  0 callbacks; a click on the clock opens `sill-bar-Virtual-0/popup` (252 x 111, presented) and a
  click on the bar's empty middle closes it. `dev/out/bar.png` is the screen with the clock
  menu open.

### Gaps found in M1 bar-ui

- **G6** `PopupSize::FitContent` never measures the popup's document: the size passed in
  `PopupPlacement` is the size (F21).
- **Q6** `Ds` renders the frame layers only for `Material::Window`; the tinted shell materials
  (Bar, Dock) need the same A/B gradient layers at the tint alpha (design/21 §3, §5) (F24).
- **Q7** quire's `HostMeasure` for Blitz documents is private to ds-native, and shell-host cannot
  provide one, so every shell-host root without one panics ("RefCell already borrowed") on its
  first `use_rect` read once shell-host polls woken tasks inside the render (shell-host
  `df9c59b`). `bar/measure.rs` repeats ds-native's twelve lines; ds-native should export it.
- **Q8** ds `Menu` exposes no item hover or item release, and calls `onclose` before `onpick`, so
  an external tracker (design/13 §13.7) cannot drive its highlight, submenus or
  press-drag-release (F22); nor can its entrance be suppressed for a hover switch (F26).
- **Q9** No ds status-item variant: `IconButton{Tool}` cannot take the bar's glyph box and size
  keys, and ds components on frame chrome use paper inks, not `--f-ink*` (F25). No wired-network
  glyph (`Icon::Link` stands in).

## Gaps reported upstream

- **Q1** quire has no menu tracking machine (F15).
- **Q2** quire has no easing evaluator for `CubicBezier` (F16).
- **Q3** quire has no `spaces.json` store (F17).
- **Q4** ds-settings' file I/O and watch are not generic over the settings file (F18).
- **Q5** The schema derive: text only by the literal type `String`; a one-variant enum cannot
  be described; a numeric newtype without an explicit `range` is read as an enum (a compile
  error naming `SchemaVariants`, not a message about the missing range).
- **G1** Debug injection: `inject::apply` (feature `debug-inject`) prints stats to stdout
  instead of returning them, and finds a surface by name only inside that module. sill's
  `debug stats` uses `HostCtx::stats`; `pointer`/`leave`/`capture` need sill's `debug` feature.
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
