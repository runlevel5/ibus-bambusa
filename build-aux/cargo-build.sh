#!/bin/sh
# Meson wrapper around `cargo build`: builds the binary then copies it to the
# path Meson expects as the custom_target output.
#
# Usage: cargo-build.sh SOURCE_ROOT TARGET_DIR OUTPUT PROFILE BIN
set -eu

SOURCE_ROOT="$1"   # workspace root (contains Cargo.toml)
TARGET_DIR="$2"    # cargo target dir (under the meson build dir)
OUTPUT="$3"        # where meson wants the built binary
PROFILE="$4"       # "release" or "debug"
BIN="$5"           # binary (and target subdir) name

# --locked enforces the committed Cargo.lock, so a packaging build cannot
# silently resolve different dependency versions than were reviewed.
if [ "$PROFILE" = "release" ]; then
    cargo build --locked --manifest-path "$SOURCE_ROOT/Cargo.toml" \
        --target-dir "$TARGET_DIR" --release --bin "$BIN"
    cp "$TARGET_DIR/release/$BIN" "$OUTPUT"
else
    cargo build --locked --manifest-path "$SOURCE_ROOT/Cargo.toml" \
        --target-dir "$TARGET_DIR" --bin "$BIN"
    cp "$TARGET_DIR/debug/$BIN" "$OUTPUT"
fi
