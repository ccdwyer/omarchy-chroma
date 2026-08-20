#!/bin/sh
# Build chromad. The plugin QML degrades to compat/chromad.sh when
# bin/chromad is missing, so a failed build is not fatal at runtime.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
SRC="$ROOT/src/chromad"
OUT="$ROOT/bin"

mkdir -p "$OUT"
chmod +x "$ROOT/compat/chromad.sh" 2>/dev/null || true

checksums() {
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$OUT" && sha256sum chromad > SHA256SUMS)
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$OUT" && shasum -a 256 chromad > SHA256SUMS)
  fi
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "build.sh: cargo not found; installing POSIX fallback as bin/chromad" >&2
  cp "$ROOT/compat/chromad.sh" "$OUT/chromad"
  chmod +x "$OUT/chromad"
  echo "build.sh: wrote $OUT/chromad (shell fallback)"
  exit 0
fi

if ! cargo build --release --manifest-path "$SRC/Cargo.toml"; then
  echo "build.sh: cargo build failed; installing POSIX fallback as bin/chromad" >&2
  cp "$ROOT/compat/chromad.sh" "$OUT/chromad"
  chmod +x "$OUT/chromad"
  echo "build.sh: wrote $OUT/chromad (shell fallback)"
  exit 0
fi

BIN="$SRC/target/release/chromad"
if [ ! -x "$BIN" ]; then
  echo "build.sh: release binary missing after cargo build" >&2
  exit 1
fi
cp "$BIN" "$OUT/chromad"
chmod +x "$OUT/chromad"
checksums
echo "build.sh: wrote $OUT/chromad"
