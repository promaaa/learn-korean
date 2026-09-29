# Omarchy / Hyprland integration

Wayland does not let applications grab global keys, so on Hyprland the compositor owns Super+Z
and forwards it to the running instance:

```
Super+Z ─▶ hyprland bind ─▶ learn-korean --toggle ─▶ single-instance ─▶ running app toggles
```

On macOS, X11 and Windows the app registers Super+Z itself (`tauri-plugin-global-shortcut`).

## Install

```sh
scripts/install-omarchy.sh
```

It builds a release binary into `~/.local/bin`, adds a desktop entry, and writes
`~/.config/hypr/learn_korean.lua` (required from `hyprland.lua`, a backup is made before the first
edit):

```lua
o.window("^learn-korean$", { tag = "-default-opacity", float = true, pin = true, center = true,
  focus_on_activate = true, border_size = 0, opacity = "1 1" })
o.bind("SUPER + Z", "Learn Korean", "~/.local/bin/learn-korean --toggle")
o.launch_on_start("~/.local/bin/learn-korean --hidden")
```

## Sync between computers

Progress (reviews, XP, focus) syncs through any folder the computers already share, e.g. MEGA:

```sh
LEARN_KOREAN_SYNC_DIR=~/MegaSync/learn-korean scripts/install-omarchy.sh
```

Run it on each computer. It writes `~/.config/dev.promaaa.learnkorean/sync.json`
(`{ "dir": "..." }`; `"device"` optionally overrides the host name) and the app then keeps
`<host name>.db` in that folder: written when the window hides or loses focus, the other computers'
files merged at start and on every Super+Z. Use one computer at a time and let the folder sync
before switching; answers made on both are kept anyway. Both need the same release (a snapshot
from another schema is skipped, see the log). Design: [ADR-006](adr/ADR-006-folder-sync.md).

## Command line

| Flag | Effect |
| --- | --- |
| *(none)* / `--show` | show and focus the window |
| `--toggle` | show if hidden or unfocused, hide otherwise |
| `--hidden` | start in the background (autostart) |

`Esc` hides the window; the process keeps running so the next Super+Z is instant.

## NVIDIA

WebKitGTK's DMA-BUF renderer crashes on the proprietary NVIDIA driver under Wayland. The app sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically when `/proc/driver/nvidia` exists.
