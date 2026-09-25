# Manual checks

Checks that need a person at the machine. They are assumed to pass until someone runs them;
each has the command to start it and what to look for. Tick the box and note the date when done.
Add new ones at the end of their section.

## On the KDE session (unlocked, at the machine)

- [ ] **CJK input in the launcher** (fcitx5 Chewing over text-input-v3; KWin has no virtual
  keyboard, so nothing can type for you). Run
  `bash dev/manual/ime-live.sh start`, click into the search field, switch fcitx5 to Chewing,
  type `su3cl3` and commit: 你好 should appear in the field. Then `bash dev/manual/ime-live.sh capture`
  (writes the capture path) and `bash dev/manual/ime-live.sh stop`. Also try a preedit that is
  deleted before commit (Backspace mid-word): no stray characters left behind.
- [ ] **Scale 2 look** on the MA270U: the desktop at 2x (the new default for 27" 4K) reads
  comfortably; text and icons sharp.
- [ ] **Brightness keys on the external monitor** (DDC/CI): after installing
  `dist/cosmic/.../system_actions` in a COSMIC session, or with `sill display brightness up|down`
  bound to keys: the MA270U's backlight follows within about a quarter second; no stuck value
  after fast repeats.
- [ ] **Monitor speakers follow the volume**: switch the default sink to the MA270U's HDMI
  audio, press volume up/down: the monitor's own volume moves too (its range is 0-50), and mute
  mutes it. Switch back to Bluetooth afterwards.

- [ ] **Power menu with the real keyboard** (real arrows, Escape and a scrim click already pass on a headless KWin through `dev/accept-real-input.sh`; this check is for a real COSMIC session) (the nested compositor never gave its Exclusive
  layer keyboard focus, shell G30): on a real COSMIC session, `sill power-menu open`, then
  arrows move the highlight, Esc closes. Do NOT press Enter on Restart/Shut Down unless you mean
  it; the dry run (`SILL_POWER_MENU_DRY_RUN=<file>` on a debug build) records instead.
- [ ] **Control center with the real keyboard** (real click and Escape already pass on a headless KWin; Tab/arrows between tiles still need a person): open it from the bar item (left of the clock);
  Tab/arrows move between tiles, Enter/Space toggle, Right on a chevron opens Wi-Fi/Bluetooth,
  Escape goes back then closes (a debug-opened popup gets no keyboard on the nested compositor).
- [ ] **Banners with a real mouse** (hover, the close button, a click, a 90 px and a 50 px drag,
  already pass under fake input on a headless KWin through
  `dev/accept-notifications-real-input.sh`; this is the feel with a real mouse and touchpad):
  `sill debug notify "Hi" --body "..." --action default=Open` on a debug build, in a session
  where sill holds `org.freedesktop.Notifications`. The card follows a drag 1:1 to the right and
  damped to the left, springs back under 80 px and flies out past it or on a flick; after a click
  on one card, the next card under the pointer should still pause and show its close button
  (FINDINGS G101 says it does not until the pointer leaves the column and comes back).
- [ ] **Notification center keys with a real keyboard** (real Escape, Down and Delete already
  pass on a headless KWin): open it from the clock; Up/Down move the ring, Return opens a
  group's "N more" or invokes a lone card's default action, Backspace (the Mac keyboard's
  Delete) clears the focused row, Escape closes.
- [ ] **The capsule level look on the light scheme** (quire Q105: the fill reads faintly on its
  track) when picking the level look from quire's sheets.

## Visual review

- [ ] **Shipped icons, app table**: the notes row (COSMIC Edit, GNOME Text Editor, Kate) and the
  photos row (Loupe, Eye of GNOME, Gwenview, Shotwell) in
  `crates/sill-surfaces/src/app_icons/shipped.rs` `DEFAULT_TABLE`; add or remove apps.
- [ ] **Icon styles in dark mode**: Colour, Muted and Monochrome in the dock
  (`dev/out/dock-icons-*.png` are light-mode only). The plate tint under Monochrome lands with
  quire's IconView plate-tint change.

## Needs a real COSMIC login (not the nested compositor)

- [ ] **Cmd+Space through Toshy** opens the launcher (the `Launcher` system action override).
- [ ] **Super shortcuts** reach the shell (the nested compositor never gets them; KDE eats them).
- [ ] **Scale policy on first sight**: a display seen for the first time gets 2x (MA270U), and a
  scale set by hand afterwards is never changed back.

## Owned by the other session (quire, shell-host)

- [ ] shell-host G100 and G101 (sill FINDINGS "M6 lane d"): `dev/accept-notifications-real-input.sh
  repro` passes both steps once shell-host moves the document's pointer on `wl_pointer.enter`
  and clears its hover when a surface is unmapped.
- [ ] shell-host spike item f at a fractional output (1.5): run on the nested compositor at
  `cosmic@1.5`; the KWin run at 2.0 left `screen_row_equals_buffer_row` false (shell-host F45).
