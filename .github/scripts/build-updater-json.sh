#!/usr/bin/env bash
# Builds the Tauri updater JSON manifest for a GitHub Release.
#
# Usage: ./build-updater-json.sh <version> <artifacts_dir>
#
# Reads *.sig file from <artifacts_dir> and creates
# <artifacts_dir>/<target>-<arch>-<version>.json
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "Usage: $0 <version> <artifacts_dir>"
  exit 1
fi

VERSION="$1"
ARTIFACTS_DIR="$2"

SIG_FILE=$(find "$ARTIFACTS_DIR" -name "*.msi.sig" | head -1)

if [ -z "$SIG_FILE" ]; then
  echo "::warning::No .msi.sig file found in $ARTIFACTS_DIR — updater will fail"
  SIGNATURE=""
  MSI_NAME="ghwtdeinitool_${VERSION}_x64_en-US.msi"
else
  SIGNATURE=$(tr -d '\n' < "$SIG_FILE")
  # Derive MSI name from sig filename by stripping .sig suffix
  MSI_NAME=$(basename "$SIG_FILE" .sig)
fi

PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%S+00:00")
OUTPUT="$ARTIFACTS_DIR/x86_64-pc-windows-msvc-x86_64-$VERSION.json"

cat > "$OUTPUT" << EOF
{
  "version": "$VERSION",
  "notes": "See the GitHub Release for details",
  "pub_date": "$PUB_DATE",
  "platforms": {
    "windows-x86_64": {
      "signature": "$SIGNATURE",
      "url": "https://github.com/$GITHUB_REPOSITORY/releases/download/v$VERSION/$MSI_NAME"
    }
  }
}
EOF

echo "::notice::Updater JSON created: $OUTPUT"