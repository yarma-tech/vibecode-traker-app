#!/usr/bin/env bash
#
# Build a Release build of Vibe Code Tracker and install it to /Applications
# (falls back to ~/Applications if /Applications isn't writable without sudo).
#
# Usage:
#   scripts/install.sh            # build, install, and launch
#   scripts/install.sh --no-open  # build and install without launching
#
set -euo pipefail

SCHEME="VibeCodeTrackerApp"
APP_NAME="VibeCodeTrackerApp.app"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Keep the generated project in sync with project.yml when xcodegen is available.
if command -v xcodegen >/dev/null 2>&1; then
  echo "==> xcodegen generate"
  xcodegen generate
fi

echo "==> Building Release (this can take a minute)…"
xcodebuild \
  -scheme "$SCHEME" \
  -configuration Release \
  -destination 'platform=macOS' \
  -derivedDataPath build \
  CODE_SIGNING_ALLOWED=NO \
  build

APP_PATH="build/Build/Products/Release/$APP_NAME"
if [ ! -d "$APP_PATH" ]; then
  echo "error: built app not found at $APP_PATH" >&2
  exit 1
fi

DEST="/Applications"
if [ ! -w "$DEST" ]; then
  DEST="$HOME/Applications"
  mkdir -p "$DEST"
  echo "==> /Applications not writable; installing to $DEST instead"
fi

# Stop a running instance so its bundle can be replaced cleanly.
pkill -f "$APP_NAME/Contents/MacOS/$SCHEME" 2>/dev/null || true
sleep 1

echo "==> Installing to $DEST/$APP_NAME"
rm -rf "${DEST:?}/$APP_NAME"
ditto "$APP_PATH" "$DEST/$APP_NAME"

# Keep the installed copy as the only Launchpad entry: unregister the build
# artifact so it doesn't show up as a duplicate (the build/ cache is preserved
# for fast incremental rebuilds).
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"
if [ -x "$LSREGISTER" ]; then
  "$LSREGISTER" -u "$APP_PATH" 2>/dev/null || true
  "$LSREGISTER" -f "$DEST/$APP_NAME" 2>/dev/null || true
fi

echo "==> Installed: $DEST/$APP_NAME"

if [ "${1:-}" != "--no-open" ]; then
  open "$DEST/$APP_NAME"
  echo "==> Launched."
fi
