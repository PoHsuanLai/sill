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
