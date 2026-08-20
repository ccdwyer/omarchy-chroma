#!/bin/sh
# Download CI-built Linux chromad binaries from GitHub Releases into bin/.
# Usage:
#   scripts/fetch-prebuilts.sh [owner/repo] [tag]
#   scripts/fetch-prebuilts.sh --verify <dir> <asset-name>
# Default repo is inferred from `git remote get-url origin`.
# Checksums are verified against the published asset filename BEFORE install.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
OUT="$ROOT/bin"
mkdir -p "$OUT"

hash_check() {
  dir=$1
  asset=$2
  sums="$dir/SHA256SUMS"
  if [ ! -f "$sums" ]; then
    echo "fetch-prebuilts.sh: SHA256SUMS missing" >&2
    return 1
  fi
  if [ ! -f "$dir/$asset" ]; then
    echo "fetch-prebuilts.sh: asset $asset missing" >&2
    return 1
  fi
  if ! grep -E "[ *]$asset\$" "$sums" >/dev/null; then
    echo "fetch-prebuilts.sh: SHA256SUMS has no line for $asset" >&2
    return 1
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dir" && grep -E "[ *]$asset\$" SHA256SUMS | sha256sum -c -)
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$dir" && grep -E "[ *]$asset\$" SHA256SUMS | shasum -a 256 -c -)
  else
    echo "fetch-prebuilts.sh: need sha256sum or shasum" >&2
    return 1
  fi
}

if [ "${1:-}" = "--verify" ]; then
  hash_check "${2:?dir}" "${3:?asset}"
  exit $?
fi

repo=${1:-}
tag=${2:-latest}
if [ -z "$repo" ]; then
  url=$(git -C "$ROOT" remote get-url origin 2>/dev/null || true)
  repo=$(printf '%s' "$url" | sed -n 's#.*github.com[:/]\([^/]*/[^/.]*\).*#\1#p' | sed 's/\.git$//')
fi
if [ -z "$repo" ]; then
  echo "fetch-prebuilts.sh: pass owner/repo (no origin remote)" >&2
  exit 1
fi

arch=$(uname -m)
case "$arch" in
  x86_64|amd64) asset=chromad-x86_64-linux ;;
  aarch64|arm64) asset=chromad-aarch64-linux ;;
  *)
    echo "fetch-prebuilts.sh: unsupported arch $arch — run ./build.sh" >&2
    exit 1
    ;;
esac

if [ "$tag" = "latest" ]; then
  api="https://github.com/$repo/releases/latest/download"
else
  api="https://github.com/$repo/releases/download/$tag"
fi

echo "fetch-prebuilts.sh: $api/$asset"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

download() {
  src=$1
  dest=$2
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$src" -o "$dest"
  else
    wget -q "$src" -O "$dest"
  fi
}

download "$api/SHA256SUMS" "$work/SHA256SUMS"
download "$api/$asset" "$work/$asset"

if ! hash_check "$work" "$asset"; then
  echo "fetch-prebuilts.sh: checksum failed; not installing" >&2
  exit 1
fi

chmod +x "$work/$asset"
cp "$work/SHA256SUMS" "$OUT/SHA256SUMS"
mv "$work/$asset" "$OUT/chromad"
trap - EXIT
rm -rf "$work"
echo "fetch-prebuilts.sh: verified $asset → $OUT/chromad"
