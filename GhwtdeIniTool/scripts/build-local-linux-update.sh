#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 99.0.0"
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to generate latest.json."
  echo "Install jq, then rerun this script."
  exit 1
fi

VERSION="$1"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][0-9A-Za-z.-]+)?$ ]]; then
  echo "Version must look like a semantic version, for example 99.0.0"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOCAL_UPDATER_DIR="$ROOT/public/updater-local"
DEB_NAME="ghwtdeinitool_${VERSION}_amd64.deb"
DEB_PATH="$ROOT/src-tauri/target/release/bundle/deb/$DEB_NAME"
SIG_PATH="$DEB_PATH.sig"
BASE_URL="http://localhost:1420/updater-local"

cd "$ROOT"

echo "Building signed Linux .deb updater artifact for version $VERSION..."
env GHWTDE_APP_VERSION="$VERSION" pnpm tauri build --bundles deb

if [ ! -f "$DEB_PATH" ]; then
  echo "Expected bundle was not created: $DEB_PATH"
  exit 1
fi

if [ ! -f "$SIG_PATH" ]; then
  echo "Expected updater signature was not created: $SIG_PATH"
  echo "Check that TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD are set."
  exit 1
fi

mkdir -p "$LOCAL_UPDATER_DIR"
cp "$DEB_PATH" "$LOCAL_UPDATER_DIR/$DEB_NAME"

SIGNATURE="$(tr -d '\r\n' < "$SIG_PATH")"
UPDATE_URL="$BASE_URL/$DEB_NAME"
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
      "linux-x86_64-deb": {
        signature: $signature,
        url: $url
      }
    }
  }' > "$LOCAL_UPDATER_DIR/latest.json"

echo "Created local Linux updater fixture:"
echo "  $LOCAL_UPDATER_DIR/$DEB_NAME"
echo "  $LOCAL_UPDATER_DIR/latest.json"
echo
echo "Use these local dev settings:"
echo "  GHWTDE_UPDATER_ENDPOINT=$BASE_URL/latest.json"
echo "  GHWTDE_APP_VERSION=<a version lower than $VERSION>"
