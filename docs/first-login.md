# First login: the sill session, with KDE as the fallback

Written 2026-10-03 for the first real login. Plasma stays installed and stays in the login
screen's session list the whole time. Nothing below changes Plasma's files.

## 1. Before you log out (about 15 minutes, in Plasma)

1. **Install the session.** From this checkout (`~/rs-wt/m-login/sill` until it is merged; its
   `../quire`, `../shell-host`, `../keycap`, `../blitz-kit` and `../palmrest` must exist next to it):

   ```sh
   rm -f ~/.cargo/bin/sill          # an old build there would shadow the installed one
   dist/install.sh --dry-run        # read it: every step is printed, nothing is changed
   dist/install.sh                  # builds release (a few minutes), asks for sudo once
   ```

   It installs `/usr/local/bin/sill`, `/usr/local/bin/sill-session`,
   `/usr/share/wayland-sessions/sill.desktop`, `/etc/systemd/user/sill-session.target`,
   `/etc/pam.d/sill` and the seed files in `/usr/local/share/sill`. It is safe to run again.
2. **Install keycap, then check it before anything else.** It replaces Toshy; the sill session has
   no Mac layout without it (Cmd stays Cmd). Follow `~/keycap/README.md`, "Install":
   `~/keycap/install.sh` builds and prints the `sudo` commands to run. Then, still in Plasma:
   `systemctl status keycap`, and try Cmd+C / Cmd+V in a text field. If the keyboard misbehaves:
   hold both Shift keys and press Esc, or `sudo systemctl stop keycap`.
3. **Only after keycap works, turn Toshy off** (it grabs the same keyboard; never run both):

   ```sh
   systemctl --user disable --now toshy-config.service toshy-cosmic-dbus.service \
     toshy-kwin-dbus.service toshy-wlroots-dbus.service toshy-session-monitor.service
   mkdir -p ~/toshy-off && mv ~/.config/autostart/Toshy_*.desktop ~/toshy-off/   # tray and kickstart
   ```

   (To bring Toshy back: move the three files back and `systemctl --user enable --now` the five units.)
4. **Chrome and Electron passwords.** Under `XDG_CURRENT_DESKTOP=COSMIC` Chrome no longer picks
   KWallet by itself. If an app asks you to sign in again, start it with `--password-store=kwallet6`
   (the LINE and Teams wrappers in `~/.local/bin`, Chrome's launcher, Cursor); that reads the same
   wallet Plasma used. `--password-store=gnome-libsecret` also works through the Secret Service but
   is a different store (you sign in once more).
5. Save your work, then log out.

## 2. At the login screen

Pick **COSMIC (sill)** from the session list (the plasmalogin menu), then log in as usual. Your
password also unlocks the wallet (`pam_kwallet`). The next login preselects it; to go back,
pick **Plasma (Wayland)** (section 5). The keyboard works here over Bluetooth, mapped by keycap.

## 3. The first 10 minutes (open COSMIC Term, or ghostty, with Cmd+Space then type it)

Go/no-go set is 1 to 7 (if 1, 3, 4 or 6 fail, log out and use section 5):

```sh
# 1 the session is up
systemctl --user is-active graphical-session.target       # active
echo $XDG_CURRENT_DESKTOP $XDG_SESSION_TYPE $XMODIFIERS   # COSMIC wayland @im=fcitx
sill polkit status; sill idle status; sill ping           # registered; {"kind":"own"}; pong
# 2 GPU: the session log says which render device it chose; sill is not at 100% CPU
journalctl -b --no-pager | grep -iE 'render device|vulkan|adapter|llvmpipe' | tail
echo "${COSMIC_RENDER_DEVICE:-unset (cosmic-comp chose)}"
top -b -n1 | head -12
# 3 launcher and keys: Cmd+Space, Super+Space, Cmd+Tab; hold a key (repeat 400 ms / 10 per s)
sill launcher toggle
# 4 the Mac keyboard (keycap)
sill keycap status; systemctl status keycap
#   Cmd+C / Cmd+V in cosmic-term, Zen, Chrome
# 5 polkit
pkexec true; echo $?                                       # prompt names the action; 0
# 6 lock (a TTY on Ctrl+Alt+F3 is your net)
loginctl lock-session                                      # unlock with your password
# 7 notifications and tray
notify-send -A default=Open hello "banner test"
# 8 Chinese input
pgrep -a fcitx5                                            # type in Zen and in the launcher
# 9 secrets
secret-tool search xdg:schema org.freedesktop.Secret.Generic 2>&1 | head -3
busctl --user status org.freedesktop.secrets | head -3     # owned (ksecretd)
#   open Chrome, Cursor, Teams: signed in? (section 1, item 4)
# 10 workspaces, screenshot, display
sill workspace next; sill workspace prev                   # also the Magic Mouse two-finger swipes
sill screenshot area
cosmic-randr list                                          # your monitor's mode and scale
sill display brightness up                                 # the monitor's backlight moves
# 11 a video in Zen: the screen must not dim; `sill idle status` shows "inhibition":"held"
```

Tick what passed in `docs/manual-checks.md`.

## 4. Known gaps (decided, not bugs)

- No Toshy: the Mac layout is keycap's. Without keycap running, Cmd is Cmd.
- `sill workspace overview` answers "unsupported" (no overview yet); `next` and `prev` work and
  stop at the first and last workspace (no wrap).
- No sink list in the control centre (G352): switch audio in COSMIC Settings > Sound or `wpctl set-default <id>`.
- Magic Mouse scroll direction and acceleration are per device in COSMIC Settings > Mouse (the
  "palmrest" devices); KDE's setting did not carry over.
- No colour management, no software night light (the monitor is dimmed and warmed over DDC/CI where it supports it), no
  screen recording app, no printer applet, no weather.
- Tray menus lose submenus (Q21). Escape does not close the control centre after a pointer press (G400).
- fcitx5 starts from XDG autostart; `GTK_IM_MODULE` and `QT_IM_MODULE` are deliberately unset.
- More in `docs/cosmic-gaps.md` and `FINDINGS.md`.

## 5. Back to KDE, and a black screen

- **Log out from sill**: power menu (power key or Super+Shift+Esc) > Log Out, then at the login
  screen pick **Plasma (Wayland)**. That is all; Plasma's config was never touched.
- **Black screen, or the login screen comes straight back**: press **Ctrl+Alt+F3**, log in on the TTY
  (with the cable or 2.4G keyboard if keycap or Bluetooth is the problem), then either
  - `systemctl restart plasmalogin` and pick Plasma at the login screen (use `sudo`), or
  - remove the sill session: `~/rs-wt/m-login/sill/dist/uninstall.sh` (`--dry-run` first;
    `--forget-seeds` also removes the key repeat and shortcut files it copied into
    `~/.config/cosmic`, if you have not changed them). The stock COSMIC and Plasma entries stay.
- **The keyboard is wrong**: hold both Shift keys and press Esc (keycap's release), or
  `sudo systemctl stop keycap`.
- **The rendering GPU** is detected at each login: with two or more GPUs, `sill session detect-gpu`
  names the one the connected monitor hangs off and the session hands it to cosmic-comp as
  `COSMIC_RENDER_DEVICE` (it prints nothing, and cosmic-comp chooses, with one GPU, no connected
  output, or outputs on several GPUs). Run it yourself to see the answer and why. To override, put a
  line in `~/.config/sill/session.env` (plain `NAME=value`, read at the next login; the file is
  never created for you), for example `COSMIC_RENDER_DEVICE=pci-0000:01:00.0` (an address from
  `ls /dev/dri/by-path`), `COSMIC_RENDER_DEVICE=` (empty: detect nothing, let cosmic-comp choose),
  or `COSMIC_DISABLE_SYNCOBJ=1` if the journal shows DRM sync-object errors. sill's own windows pick
  their GPU separately through wgpu (`WGPU_ADAPTER_NAME=<part of the adapter's name>` in the same
  file overrides it); if the shell is slow or the CPU is busy, try that first.

## 6. Where the logs are

```sh
journalctl -b --no-pager | grep -iE 'sill|cosmic-comp' | tail -100   # the session script, compositor, shell
journalctl --user -b -u sill-session.target -u xdg-desktop-portal -u xdg-desktop-portal-cosmic
journalctl --user -b -u palmrest -u 'app-org.fcitx.Fcitx5@autostart'
journalctl -b -u keycap                                              # system unit
```

The session script restarts its components (settings daemon, portal, sill) and gives up after five
quick exits, logging `<name> keeps exiting; not restarting it`.
