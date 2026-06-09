#!/bin/sh
# Build the .deb. dpkg-buildpackage requires debian/ at the root of the source
# tree, so assemble a clean tree from `git archive` plus this debian/ directory.
set -eu

ROOT="$(git rev-parse --show-toplevel)"
VERSION=$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
TMP="$(mktemp -d)"
DEST="$TMP/ibus-bambusa-${VERSION}"

git -C "$ROOT" archive --prefix="ibus-bambusa-${VERSION}/" HEAD | tar -x -C "$TMP"
cp -r "$ROOT/build/deb/debian" "$DEST/debian"
( cd "$DEST" && dpkg-buildpackage -us -uc -b )

mkdir -p "$ROOT/build/deb/out"
mv "$TMP"/*.deb "$ROOT/build/deb/out/" 2>/dev/null || true
rm -rf "$TMP"

echo "Debs are in build/deb/out/"
