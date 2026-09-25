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

- [ ] **Power menu with the real keyboard** (the nested compositor never gave its Exclusive
  layer keyboard focus, shell G30): on a real COSMIC session, `sill power-menu open`, then
  arrows move the highlight, Esc closes. Do NOT press Enter on Restart/Shut Down unless you mean
  it; the dry run (`SILL_POWER_MENU_DRY_RUN=<file>` on a debug build) records instead.
- [ ] **The bar's "Shut Down…" item** opens the power menu (G31: a scripted click on the popup
  item did not open it on the nested compositor; unit-tested only).

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

- [ ] shell-host spike item f at a fractional output (1.5): run on the nested compositor at
  `cosmic@1.5`; the KWin run at 2.0 left `screen_row_equals_buffer_row` false (shell-host F45).
