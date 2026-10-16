# First login: the sill (casement) session, with KDE as the fallback

Written 2026-10-03 for the first real login, rewritten 2026-10-14 for the casement session. Plasma stays installed and stays in the login
screen's session list the whole time. Nothing below changes Plasma's files.

## 1. Before you log out (about 30 minutes, in Plasma)

1. **Save your work and know the way back.** The keycap emergency chord is both Shift keys + Esc
   (`sudo systemctl stop keycap` also works). Keep a text console in mind: Ctrl+Alt+F3. Ctrl+Alt+F1 to F12 switch text consoles; with the keyboard in
   Mac mode the F-keys may need Fn, and `sudo chvt 3` from a terminal always works. For anything before
   the OS (MOK, BIOS) the K5 Max must be on Cable or 2.4G, not Bluetooth.
2. **Bring the sibling checkouts up to date.** The install builds with `--locked`, and `Cargo.lock`
   was resolved against the current master of every repo next to `~/sill`: `almanac`, `anyview`,
   `blitz-kit`, `cua`, `docket`, `keycap`, `porter`, `quire`, `shell-host`, plus `stoker` and
   `casement-protocols` (reached through them). `palmrest` is not needed to build. A stale one
   (`anyview` was 11 commits behind once) stops the build with a raw cargo error, so check first:

   ```sh
   for r in anyview quire shell-host keycap blitz-kit almanac cua docket porter stoker casement-protocols; do
     git -C ~/$r pull --ff-only || echo "NOT FAST-FORWARD: $r"; done
   git -C ~/anyview pull --ff-only                          # the one that was behind
   cd ~/sill && git status --short && git log --oneline -1  # clean, master
   cargo metadata --locked --format-version 1 >/dev/null && echo LOCK OK   # must print LOCK OK
   ```

   `dist/install.sh` runs the same `cargo metadata --locked` check as its step 2 and, when it fails,
   names the siblings that are behind and the `git -C <path> pull --ff-only` that fixes each; follow that
   advice and run it again. Install only when `sill` and `quire` are both on a merged master (a
   quire merge breaks sill until sill follows it).
3. **Remove a stale binary that would shadow the install:** `rm -f ~/.cargo/bin/sill`.
4. **Install sill and the session.** The login entry "sill (casement)" and its script come from the desktop repo (`~/desktop/install.sh --dry-run` prints the plan: porter, detent, sill, keycap and the rest, then the session files). sill's own part, from `~/sill`:

   ```sh
   dist/install.sh --dry-run        # read it: every step is printed, nothing is changed
   dist/install.sh                  # builds release (a few minutes), asks for root once
   ```

   Options: `--prefix DIR` (default `/usr/local`), `--destdir DIR` (stage under DIR for packaging; nothing
   is escalated), `--no-build` (install the release binary already built). Root comes from `$SUDO` if
   set, else `sudo`, `doas` or `pkexec`; with none of them the commands that need root are printed at the
   end for you to run. The install links with your system's own linker: it builds from `/` with
   `--manifest-path`, so the repo's `.cargo/config.toml` (`clang` and `mold`, for development) is not read.
   Nothing is installed for the linker.

   It installs `/usr/local/bin/sill`, `/etc/pam.d/sill`, the intents manifest
   `/usr/local/share/quire/intents/org.quire.Shell.toml`, the skills and the settings schema; it also
   copies the keycap layer to `~/.config/keycap/keycap.toml` when you have none (an existing file is
   never touched). It is safe to run again. Then install the session from `~/desktop` (it writes
   `desktop-session` and the `sill (casement)` entry in `wayland-sessions`). Confirm:

   ```sh
   ls /usr/local/bin/sill /etc/pam.d/sill ~/.config/keycap/keycap.toml
   ls /usr/share/wayland-sessions/ | grep -i casement
   ```
   **System services sill uses (all optional).** Each feature below works only while its service runs
   on the system bus, and hides quietly when the service is absent (no error, the item is just not
   there). Fedora, Debian and Ubuntu desktops have most of them already; Arch installs none by default.

   | Feature | Service | dnf | apt | pacman |
   |---|---|---|---|---|
   | Wi-Fi and network menu | NetworkManager | `NetworkManager` | `network-manager` | `networkmanager` |
   | Bluetooth | BlueZ | `bluez` | `bluez` | `bluez` |
   | Battery | UPower | `upower` | `upower` | `upower` |
   | Power profile | power-profiles-daemon | `power-profiles-daemon` | `power-profiles-daemon` | `power-profiles-daemon` |
   | Your picture and name | AccountsService | `accountsservice` | `accountsservice` | `accountsservice` |

   Enable what you install (`sudo systemctl enable --now NetworkManager bluetooth upower power-profiles-daemon accounts-daemon`;
   the unit names differ little between distributions). Two more programs are needed at run time:
   `curl` (cover art, currency rates) and `xdg-utils` (opening links), the same package names on all
   three. External monitor brightness needs the `i2c-dev` module and an ACL on `/dev/i2c-*`: the install
   adds both unless a ddcutil package already does (`ddcutil` on all three).
5. **keycap** replaces Toshy; the sill session has no Mac layout without it (Cmd stays Cmd). It is
   installed and running on this machine; if not, `~/keycap/install.sh --dry-run`, then
   `~/keycap/install.sh` (it builds and installs with sudo; see `~/keycap/README.md`, "Install").
   Check `systemctl status keycap` and Cmd+C / Cmd+V in a text field in Plasma.

   `/etc/keycap/keycap.toml` is the Plasma overlay and is shared by every session: it writes
   Cmd+Space as Alt+Space (KRunner), which opens nothing in sill. So the sill session has its own layer,
   `~/.config/keycap/keycap.toml` (copied from `dist/keycap/keycap.toml` by the install). sill forwards
   that file to keycap and keycap lays it over the machine file, so in sill Cmd+Space is Super+Space and
   Cmd+Shift+3/4/5 is Print (`sill screenshot area`); Plasma never forwards it and keeps `alt+space`.
   It also sends Cmd+Tab and Shift+Cmd+Tab to the app switcher (Super+Tab, with Super held while Cmd is held, so the
   panel stays up), Shift+Cmd+Q to the power menu and Option+Cmd+Esc to Force Quit. If you installed before these
   were added, the install does not touch your file: it prints the rules yours lacks and the one command to
   take the new file (your old one is kept as `keycap.toml.bak`).
   Check offline (`keycap replay` reads no device):

   ```sh
   printf '0 press 1 apple leftmeta\n10 press 1 apple space\n20 release 1 apple space\n30 release 1 apple leftmeta\n' > /tmp/cmd-space.txt
   keycap replay /tmp/cmd-space.txt                           # built-in mapping: +leftmeta +space (Super+Space)
   keycap replay --config /etc/keycap/keycap.toml /tmp/cmd-space.txt   # the Plasma overlay alone: +leftalt +space
   ```

   Replay takes one file, so the layered result is checked after login: Cmd+Space opens the launcher
   (section 3, item 2).
6. **If Toshy is still around, turn it off** (it grabs the same keyboard; never run both). It is already
   gone on this machine; `systemctl --user list-unit-files 'toshy*'` and `ls ~/.config/autostart` tell you:

   ```sh
   systemctl --user disable --now toshy-config.service \
     toshy-kwin-dbus.service toshy-wlroots-dbus.service toshy-session-monitor.service
   mkdir -p ~/toshy-off && mv ~/.config/autostart/Toshy_*.desktop ~/toshy-off/   # tray and kickstart
   ```
7. **Chrome and Electron passwords.** Under `XDG_CURRENT_DESKTOP=quire` Chrome no longer picks
   KWallet by itself. If an app asks you to sign in again, start it with `--password-store=kwallet6`
   (the LINE and Teams wrappers in `~/.local/bin`, Chrome's launcher, Cursor); that reads the same
   wallet Plasma used. `--password-store=gnome-libsecret` also works through the Secret Service but
   is a different store (you sign in once more).
8. **Mouse settings.** Plasma's per-mouse settings are not copied. casement's settings
   (`~/.config/casement/settings.toml`, `[pointer] natural_scroll`) hold the mouse options, and natural
   scroll is on by default. Change it there if you want the other direction.
9. Do not install `dist/sill.service` yourself (the desktop repo's installer puts it under `/etc/systemd/user`, root-held, and the session starts it; a copy under `~/.config` is not trusted) and do not enable `voiced.service` (see "The agent stack" in
   section 3). `dist/udev` and `dist/modules-load.d` (external monitor brightness) are installed by
   `dist/install.sh` itself, and only when no ddcutil udev rule exists on the machine.
10. Log out of Plasma.

## 2. At the login screen

Pick **sill (casement)** from the session list (the plasmalogin menu), then log in as usual. Your
password also unlocks the wallet (`pam_kwallet`). The next login preselects it; to go back,
pick **Plasma (Wayland)** (section 5). The keyboard works here over Bluetooth, mapped by keycap.

## 3. The first 10 minutes (open foot, or ghostty, with Cmd+Space then type it)

Go/no-go set is 1 to 8 (if 1, 2, 4, 6 or 7 fail, log out and use section 5):

```sh
# 1 the session is up
systemctl --user is-active graphical-session.target; echo $XDG_CURRENT_DESKTOP $XDG_SESSION_TYPE $XMODIFIERS
sill ping; sill polkit status; sill idle status        # pong; registered; {"kind":"own"}
journalctl -b --no-pager | grep -E 'desktop-session|guard' | tail     # no quick exits; cat $XDG_RUNTIME_DIR/desktop-session/guard.log is empty
# 2 launcher by keyboard (G1/G2/L2): Cmd+Space opens it, again closes it; type "term", Enter; also Super+Space
#   (Cmd+Space does nothing: Alt+Space is being sent, so ~/.config/keycap/keycap.toml is missing; section 1, item 5)
# 3 GPU (B3): which device did it pick, is sill quiet
journalctl -b --no-pager | grep -iE 'render device|adapter|vulkan|llvmpipe' | tail
top -b -n1 | head -12                                    # sill and casement not busy at idle
# 4 keyboard: Cmd+C/Cmd+V in foot, Zen, Chrome; Ctrl+C still interrupts in the terminal; hold a key (400 ms / 10 per s)
keycap status; sill keycap status                        # grabbed; connected
# 5 polkit
pkexec true; echo $?                                     # prompt names the action; 0
# 6 lock, with a TTY ready
loginctl lock-session                                    # unlock with your password; try one wrong one
# 7 power key and log out: the power key opens sill's menu (does not power off): systemd-inhibit --list shows the power key held
# 8 notifications and tray
notify-send -A default=Open hello "banner test"
```

Then, in this order, tick them in `docs/manual-checks.md`:

```sh
# 9  Cmd+Tab: tap switches to the previous app, hold shows the panel
# 10 Chinese input: pgrep -a fcitx5; type in Zen and in the launcher (Caps Lock switches input: keycap maps it to Ctrl+Space)
# 11 secrets: secret-tool search xdg:schema org.freedesktop.Secret.Generic 2>&1 | head -3 ; open Chrome, Cursor, Teams: signed in?
#    busctl --user status org.freedesktop.secrets | head -3   # owned (ksecretd)
#    (if not: start them with --password-store=kwallet6; section 1, item 7)
# 12 display: casementctl debug output list (3840x2160, scale 2); sill display brightness up (the BenQ moves, OSD shows)
# 13 video in Zen: the screen must not dim; sill idle status shows "inhibition":"held"
# 14 power menu offers (N14): the menu lists Log Out, Restart, Shut Down (do not press Restart or Shut Down)
# 15 workspaces and the mouse: sill workspace next; sill workspace prev; sill workspace overview (Mission Control opens); scroll the Magic Mouse: direction (section 1, item 8; else Settings > Mouse)
# 16 screenshot and share: sill screenshot area; share a screen in Zen or Teams; plug a USB drive, open it in your file manager
# 17 quire apps: open mailo and anyview: which adapter do they log, does a second window close cleanly (X5)
# 18 SELinux: sudo ausearch -m avc -ts recent | head    # expect nothing about sill, ksecretd, pam_kwallet_init
# 19 leave it for an evening: free -h, top, journalctl -b -p warning (L16)
```

Write the results next to the boxes in `docs/manual-checks.md`, and the surprises in a note for the sill session.

### The agent stack (none of it blocks the session)

- Do not install or enable `voiced.service`: its binary is still a skeleton that exits 2.
- Summon the companion once with no model installed (`sill companion summon`): it must
  end with a plain failure within a few seconds, and the session keeps working.
- `journalctl --user -u memoryd -b`: no crash loop. With the keyring locked or absent, memoryd reports the
  Space locked and intentd counts audit records as lost; that is expected.
- `journalctl --user -u cuad -b`: idle, no error loop; `cua.enabled` stays off.
- Any hang or loop: save the journal lines for the agent session.

## 4. Known gaps (decided, not bugs)

- No Toshy: the Mac layout is keycap's. Without keycap running, Cmd is Cmd. A lone Cmd tap has no sender yet (the companion's
  Cmd double-tap); use `sill companion summon`.
- `sill workspace next` and `prev` stop at the first and last workspace (no wrap).
- Option+Cmd+Esc (Force Quit Applications) does nothing until casement runs `sill force-quit toggle`
  for its Force Quit chord (today the chord is swallowed by a command that does nothing). Run
  `sill force-quit toggle` from a terminal to see the panel.
- No sink list in the control centre (G352): switch audio in Settings > Sound or `wpctl set-default <id>`.
- Mouse settings are casement's (`~/.config/casement/settings.toml`, section 1, item 8), or per device in
  Settings > Mouse (the "palmrest" devices).
- Night Shift: the control centre's Display module switches casement's software night light on until
  tomorrow and sets its warmth; the schedule (times, "always") is casement's `[night_light]`, set in
  Settings. Sunset to Sunrise is not offered (sill has no location). Where `display.night_warmth` is
  `hardware` the monitors are warmed over DDC/CI instead and casement's night light is kept off.
- No colour management, no screen recording app, no printer applet, no weather.
- Escape does not close the control centre after a pointer press on empty space (G400; waits on quire's
  shell-surface click focus).
- fcitx5 starts from XDG autostart; `GTK_IM_MODULE` and `QT_IM_MODULE` are deliberately unset.
- More in `FINDINGS.md` ("What casement still has to provide").

## 5. Back to KDE, and a black screen

- **Log out from sill**: power menu (power key or Super+Shift+Esc) > Log Out, then at the login
  screen pick **Plasma (Wayland)**. That is all; Plasma's config was never touched.
- **Black screen, or the login screen comes straight back**: press **Ctrl+Alt+F3** (or `sudo chvt 3`), log in on the TTY
  (with the cable or 2.4G keyboard if keycap or Bluetooth is the problem), then either
  - `systemctl restart plasmalogin` and pick Plasma at the login screen (use `sudo`), or
  - remove the sill session: `~/sill/dist/uninstall.sh` (`--dry-run` first;
    `--forget-seeds` also removes the keycap layer in `~/.config/keycap`, if you have not changed it). The Plasma entry stays.
- **The keyboard is wrong**: hold both Shift keys and press Esc (keycap's release), or
  `sudo systemctl stop keycap`.
- **The rendering GPU.** sill's own windows render on the same GPU as the compositor, on any
  hardware: the shell asks the compositor which device it renders on (the `main_device` of
  `zwp_linux_dmabuf_v1`'s default feedback, which needs version 4) and takes the wgpu adapter with
  that PCI vendor and device id, so frames cross no GPU boundary. If the compositor does not say,
  or no adapter matches, wgpu's own order applies (a discrete GPU first), and any adapter that fails
  a first test frame is skipped. The journal line `shell-host: adapter <name> ...` names the one
  chosen. To override, put `WGPU_ADAPTER_NAME=<part of the adapter's name>` (for example `AMD` or
  `NVIDIA`) in `~/.config/desktop-session/session.env` (plain `NAME=value`, read at the next login;
  the file is never created for you): it wins over the detection. If the shell is slow or the CPU is
  busy, check that line first.

## 6. Where the logs are

```sh
journalctl -b --no-pager | grep -iE 'sill|casement|desktop-session' | tail -100   # the session script, compositor, shell
journalctl --user -b -u casement.service -u desktop-session.target -u xdg-desktop-portal
journalctl --user -b -u palmrest -u 'app-org.fcitx.Fcitx5@autostart'
journalctl -b -u keycap                                              # system unit
cat "$XDG_RUNTIME_DIR"/desktop-session/guard.log                     # the crash-loop guard's decisions
```

The session script runs casement as a user unit and gives up after three quick exits (then
`casement --safe`, then it ends the session); see the desktop repo's `session/desktop-session`.
