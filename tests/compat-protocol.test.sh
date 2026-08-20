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

: > "$LOG"
out=$(CHROMA_NO_PYTHON=1 serve '{"cmd":"palette","source":"window"}' '{"cmd":"quit"}')
if printf '%s' "$out" | grep -q '"event":"error"' && printf '%s' "$out" | grep -q 'python3'; then
  ok "palette without python reports degraded capability"
else
  bad "no-python palette: $out"
fi
if printf '%s' "$out" | grep -q '#111111'; then
  bad "palette without python must not invent a synthetic palette"
else
  ok "palette without python has no synthetic swatches"
fi

# Overlay root methods (shell call contract)
for fn in "function pick(arg)" "function palette(arg)" "function revert(arg)" "function status(arg)"; do
  grep -q "$fn" "$ROOT/Overlay.qml" && ok "Overlay root $fn" || bad "missing Overlay $fn"
done

grep -q 'if (!root.themeLive)' "$ROOT/Overlay.qml" && ok "preview snapshots when not live" || bad "missing themeLive snapshot"
grep -q 'clearThemeSession' "$ROOT/Overlay.qml" && ok "revert clears theme session" || bad "missing clearThemeSession"
grep -q 'moduleName: "io.github.chris.chroma"' "$ROOT/Overlay.qml" && ok "overlay moduleName" || bad "overlay moduleName missing"
grep -q 'AnchorChanges' "$ROOT/Overlay.qml" && ok "HUD uses AnchorChanges" || bad "HUD anchors"
grep -q 'chromad not ready' "$ROOT/Overlay.qml" && ok "theme apply fails closed without chromad" || bad "theme fail-closed"

printf '%s\n' "$fail failed"
[ "$fail" -eq 0 ]
