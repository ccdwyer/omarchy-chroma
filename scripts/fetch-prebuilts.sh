#!/bin/sh
# Download CI-built Linux chromad binaries from GitHub Releases into bin/.
# Usage: scripts/fetch-prebuilts.sh [owner/repo] [tag]
# Default repo is inferred from `git remote get-url origin`.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
OUT="$ROOT/bin"
mkdir -p "$OUT"

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
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$api/$asset" -o "$tmp"
  curl -fsSL "$api/SHA256SUMS" -o "$OUT/SHA256SUMS" || true
else
  wget -q "$api/$asset" -O "$tmp"
  wget -q "$api/SHA256SUMS" -O "$OUT/SHA256SUMS" || true
fi
chmod +x "$tmp"
mv "$tmp" "$OUT/chromad"
trap - EXIT
echo "fetch-prebuilts.sh: wrote $OUT/chromad"
if [ -f "$OUT/SHA256SUMS" ] && command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT" && sha256sum -c SHA256SUMS --ignore-missing) || echo "fetch-prebuilts.sh: checksum file present; verify on Linux" >&2
fi
