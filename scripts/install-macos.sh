#!/usr/bin/env bash
# Build learn-korean and install it on macOS:
#   - /Applications/learn-korean.app, global shortcut Ctrl+Option+Z (registered by the app)
#   - started hidden at login by a LaunchAgent, restarted now so the new build is used
#   - with LEARN_KOREAN_SYNC_DIR set, progress syncs through that shared folder (docs/omarchy.md);
#     LEARN_KOREAN_DEVICE names this computer's snapshot (default: the host name)
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app="/Applications/learn-korean.app"
label="dev.promaaa.learnkorean"
app_config_dir="$HOME/Library/Application Support/$label"
agent="$HOME/Library/LaunchAgents/$label.plist"

cd "$repo"
pnpm install --frozen-lockfile
pnpm --filter client tauri build --bundles app

rm -rf "$app"
cp -R target/release/bundle/macos/learn-korean.app "$app"

if [[ -n "${LEARN_KOREAN_SYNC_DIR:-}" ]]; then
  mkdir -p "$app_config_dir"
  jq -n --arg dir "$LEARN_KOREAN_SYNC_DIR" --arg device "${LEARN_KOREAN_DEVICE:-}" \
    '{dir: $dir} + (if $device == "" then {} else {device: $device} end)' \
    >"$app_config_dir/sync.json"
  echo "Progress sync: $LEARN_KOREAN_SYNC_DIR"
fi

mkdir -p "$(dirname "$agent")"
cat >"$agent" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$label</string>
  <key>ProgramArguments</key>
  <array>
    <string>$app/Contents/MacOS/learn-korean</string>
    <string>--hidden</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
EOF

# Restart the background instance so the new build is used. An instance launched by hand is not
# launchd's: stop it too, or the new one hands its arguments to it and exits (single instance).
domain="gui/$(id -u)"
launchctl bootout "$domain/$label" 2>/dev/null || true
pkill -x learn-korean || true
launchctl bootstrap "$domain" "$agent"
echo "Installed. Press Ctrl+Option+Z."
