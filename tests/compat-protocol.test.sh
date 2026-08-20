#!/bin/sh
# Protocol tests for compat/chromad.sh using fake grim/hyprctl.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
FIX="$ROOT/tests/fixtures"
PATH="$FIX/bin:$PATH"
export PATH
export CHROMA_FIX="$FIX"
chmod +x "$FIX/bin/grim" "$FIX/bin/hyprctl"

fail=0
ok() { printf 'ok  %s\n' "$1"; }
bad() { printf 'FAIL %s\n' "$1" >&2; fail=$((fail + 1)); }

SHM=$(mktemp -d)
export CHROMA_SHM_DIR="$SHM"
LOG=$(mktemp)
export CHROMA_GRIM_LOG="$LOG"

serve() {
  printf '%s\n' "$@" | sh "$ROOT/compat/chromad.sh" --serve
}

out=$(serve '{"cmd":"freeze"}' '{"cmd":"quit"}')
printf '%s' "$out" | grep -q '"event":"frozen"' && ok "freeze emits frozen" || bad "freeze frozen: $out"
printf '%s' "$out" | grep -q '"event":"frame"' && ok "freeze emits frame" || bad "freeze frame"
if printf '%s' "$out" | grep -q '"event":"pick"'; then
  bad "freeze must not emit pick"
else
  ok "freeze does not emit pick"
fi

: > "$LOG"
out=$(serve '{"cmd":"palette","source":"window"}' '{"cmd":"quit"}')
printf '%s' "$out" | grep -q '"event":"palette"' && ok "palette event" || bad "palette event: $out"
if grep -q '400,200 1520x800' "$LOG"; then
  ok "palette grim uses window∩monitor geometry"
else
  bad "palette grim geom: $(cat "$LOG")"
fi

SHM2=$(mktemp -d)
export CHROMA_SHM_DIR="$SHM2"
serve '{"cmd":"quit"}' >/dev/null
if [ -d "$SHM2" ]; then
  bad "shm dir still present after quit"
else
  ok "shm dir removed on quit"
fi

# Overlay root methods (shell call contract)
for fn in "function pick(arg)" "function palette(arg)" "function revert(arg)" "function status(arg)"; do
  grep -q "$fn" "$ROOT/Overlay.qml" && ok "Overlay root $fn" || bad "missing Overlay $fn"
done

printf '%s\n' "$fail failed"
[ "$fail" -eq 0 ]
