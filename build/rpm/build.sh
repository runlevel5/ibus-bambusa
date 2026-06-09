#!/bin/sh
# Build the RPM from a clean source tarball (run from anywhere in the repo).
set -eu

ROOT="$(git rev-parse --show-toplevel)"
VERSION=$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
TOP="$ROOT/build/rpm/rpmbuild"

mkdir -p "$TOP/SOURCES"
git -C "$ROOT" archive --prefix="ibus-bambusa-${VERSION}/" \
    -o "$TOP/SOURCES/ibus-bambusa-${VERSION}.tar.gz" HEAD
rpmbuild --define "_topdir $TOP" -bb "$ROOT/build/rpm/ibus-bambusa.spec"

echo "RPMs are in $TOP/RPMS/"
