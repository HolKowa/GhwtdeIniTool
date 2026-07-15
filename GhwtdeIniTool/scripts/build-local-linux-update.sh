#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 0 ]; then
  echo "Usage: $0"
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to generate latest.json."
  echo "Install jq, then rerun this script."
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="$(jq -r '.version // empty' "$ROOT/src-tauri/tauri.conf.json")"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][0-9A-Za-z.-]+)?$ ]]; then
  echo "Version in src-tauri/tauri.conf.json must be a semantic version, for example 99.0.0"
  exit 1
fi

LOCAL_UPDATER_DIR="$ROOT/public/updater-local"
APPIMAGE_NAME="ghwtdeinitool_${VERSION}_amd64.AppImage"
APPIMAGE_PATH="$ROOT/src-tauri/target/release/bundle/appimage/$APPIMAGE_NAME"
SIG_PATH="$APPIMAGE_PATH.sig"
BASE_URL="http://localhost:1420/updater-local"

cd "$ROOT"

echo "Building signed Linux AppImage updater artifact for version $VERSION..."
pnpm tauri build --bundles appimage

if [ ! -f "$APPIMAGE_PATH" ]; then
  echo "Expected bundle was not created: $APPIMAGE_PATH"
  exit 1
fi

if [ ! -f "$SIG_PATH" ]; then
  echo "Expected updater signature was not created: $SIG_PATH"
  echo "Check that TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD are set."
  exit 1
fi

mkdir -p "$LOCAL_UPDATER_DIR"
cp "$APPIMAGE_PATH" "$LOCAL_UPDATER_DIR/$APPIMAGE_NAME"

SIGNATURE="$(tr -d '\r\n' < "$SIG_PATH")"
UPDATE_URL="$BASE_URL/$APPIMAGE_NAME"
PUB_DATE="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

jq -n \
  --arg version "$VERSION" \
  --arg notes "## v$VERSION local Linux updater test" \
  --arg pub_date "$PUB_DATE" \
  --arg signature "$SIGNATURE" \
  --arg url "$UPDATE_URL" \
  '{
    version: $version,
    notes: $notes,
    pub_date: $pub_date,
    platforms: {
      "linux-x86_64": {
        signature: $signature,
        url: $url
      },
      "linux-x86_64-appimage": {
        signature: $signature,
        url: $url
      }
    }
  }' > "$LOCAL_UPDATER_DIR/latest.json"

echo "Created local Linux updater fixture:"
echo "  $LOCAL_UPDATER_DIR/$APPIMAGE_NAME"
echo "  $LOCAL_UPDATER_DIR/latest.json"
echo
echo "Use these local dev settings:"
echo "  GHWTDE_UPDATER_ENDPOINT=$BASE_URL/latest.json"
echo "  GHWTDE_APP_VERSION=<a version lower than $VERSION>"
