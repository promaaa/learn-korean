#!/usr/bin/env bash
# Build learn-korean and integrate it with Omarchy/Hyprland:
#   - binary in ~/.local/bin, desktop entry and icon in ~/.local/share
#   - Super+Z bound to `learn-korean --toggle`, floating pinned overlay window rule
#   - started hidden at login so the first Super+Z is instant
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
hypr_dir="${XDG_CONFIG_HOME:-$HOME/.config}/hypr"

cd "$repo"
pnpm install --frozen-lockfile
pnpm --filter client tauri build --no-bundle

install -Dm755 target/release/learn-korean "$bin_dir/learn-korean"
install -Dm644 apps/client/src-tauri/icons/128x128.png "$data_dir/icons/hicolor/128x128/apps/learn-korean.png"
install -Dm644 /dev/stdin "$data_dir/applications/learn-korean.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Learn Korean
Comment=Keyboard-first Korean trainer
Exec=$bin_dir/learn-korean --toggle
Icon=learn-korean
Categories=Education;
StartupWMClass=learn-korean
EOF

if [[ -d "$hypr_dir" && -f "$hypr_dir/hyprland.lua" ]]; then
  install -Dm644 /dev/stdin "$hypr_dir/learn_korean.lua" <<EOF
-- Managed by learn-korean/scripts/install-omarchy.sh
o.window("^learn-korean\$", {
  tag = "-default-opacity",
  float = true,
  pin = true,
  center = true,
  focus_on_activate = true,
  border_size = 0,
  opacity = "1 1",
})
o.bind("SUPER + Z", "Learn Korean", "$bin_dir/learn-korean --toggle")
o.launch_on_start("$bin_dir/learn-korean --hidden")
EOF
  if ! grep -q 'require("hypr.learn_korean")' "$hypr_dir/hyprland.lua"; then
    cp "$hypr_dir/hyprland.lua" "$hypr_dir/hyprland.lua.bak.$(date +%s)"
    printf '\n-- learn-korean overlay (Super+Z)\nrequire("hypr.learn_korean")\n' >>"$hypr_dir/hyprland.lua"
  fi
  hyprctl reload >/dev/null
  errors="$(hyprctl configerrors)"
  if [[ -n "${errors//[[:space:]]/}" ]]; then
    echo "Hyprland reported config errors:" >&2
    echo "$errors" >&2
    exit 1
  fi
fi

# Restart the background instance so the new binary is used.
pkill -x learn-korean || true
setsid -f "$bin_dir/learn-korean" --hidden >/dev/null 2>&1
echo "Installed. Press Super+Z."
