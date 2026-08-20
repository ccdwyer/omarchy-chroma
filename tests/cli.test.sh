#!/bin/sh
# Off-device CLI checks for chromad and the grim-compat fallback.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
FIX="$ROOT/tests/fixtures"
fail=0

ok() { printf 'ok  %s\n' "$1"; }
bad() { printf 'FAIL %s\n' "$1" >&2; fail=$((fail + 1)); }

# compat --capabilities speaks hello JSON
out=$(sh "$ROOT/compat/chromad.sh" --capabilities)
printf '%s' "$out" | grep -q '"pickMode":true' && ok "compat hello is pick-mode" || bad "compat hello"
printf '%s' "$out" | grep -q '"backend":"grim"' && ok "compat backend grim" || bad "compat backend"

# map-coords via cargo if the crate is present
BIN=""
for cand in \
  "$ROOT/src/chromad/target/debug/chromad" \
  "$ROOT/src/chromad/target/release/chromad" \
  "$ROOT/bin/chromad"
do
  if [ -x "$cand" ]; then
    case "$(file -b "$cand" 2>/dev/null || true)" in
      *shell*|*text*) ;;
      *) BIN="$cand"; break ;;
    esac
    # file(1) missing: treat as binary if it has no shebang
    if [ -z "$BIN" ] && ! head -n 1 "$cand" | grep -q '^#!'; then
      BIN="$cand"
      break
    fi
  fi
done

if [ -n "$BIN" ]; then
  mapped=$("$BIN" --map-coords --monitors "$FIX/monitors-1.5x.json" --x 100 --y 50)
  printf '%s' "$mapped" | grep -q '"physicalX":150' && ok "map-coords 1.5x" || bad "map-coords 1.5x: $mapped"

  pal=$("$BIN" --palette "$FIX/split.ppm")
  printf '%s' "$pal" | grep -q '"colors"' && ok "palette from ppm" || bad "palette from ppm"

  pix=$("$BIN" --pixel "$FIX/split.ppm" 0 0)
  printf '%s' "$pix" | grep -q '"r":220' && ok "pixel 0,0 is red-ish" || bad "pixel: $pix"

  rec=$("$BIN" --check-recursion --image "$FIX/split.ppm" --sentinel "#ff2bd6" || true)
  printf '%s' "$rec" | grep -q '"hit":false' && ok "no-recursion on split.ppm" || bad "recursion: $rec"
else
  ok "chromad binary skipped (not built as a real ELF/Mach-O)"
fi

printf '%s\n' "$fail failed"
[ "$fail" -eq 0 ]
