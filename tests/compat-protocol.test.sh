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

: > "$LOG"
out=$(serve '{"cmd":"palette","source":"monitor"}' '{"cmd":"quit"}')
printf '%s' "$out" | grep -q '"event":"palette"' && ok "monitor palette event" || bad "monitor palette event: $out"
if grep -q -- '-o DP-1' "$LOG"; then
  ok "monitor palette uses focused output"
else
  bad "monitor palette grim: $(cat "$LOG")"
fi

: > "$LOG"
out=$(CHROMA_NO_PYTHON=1 serve '{"cmd":"pick"}' '{"cmd":"quit"}')
if printf '%s' "$out" | grep -q '"event":"error"' && ! printf '%s' "$out" | grep -q '#808080'; then
  ok "pick without python is a protocol error"
else
  bad "no-python pick: $out"
fi

: > "$LOG"
MON_SAVE=${CHROMA_MONITORS:-}
export CHROMA_MONITORS="$FIX/monitors-left.json"
out=$(serve '{"cmd":"cursor","x":-1000,"y":100}' '{"cmd":"pick"}' '{"cmd":"quit"}')
if grep -q -- '-1064,36 128x128' "$LOG"; then
  ok "signed region geom on left-of-origin monitor"
else
  bad "signed geom: $(cat "$LOG") / $out"
fi
if grep -q '0,36 128x128' "$LOG" || grep -q '0,0 128x128' "$LOG"; then
  bad "region geom clamped to global zero: $(cat "$LOG")"
else
  ok "region geom not clamped to global zero"
fi
if [ -n "$MON_SAVE" ]; then
  export CHROMA_MONITORS="$MON_SAVE"
else
  unset CHROMA_MONITORS
fi

# Overlay root methods (shell call / IpcHandler contract)
for fn in "function pick(arg)" "function palette(arg)" "function revert(arg)" "function status(arg)"; do
  grep -q "$fn" "$ROOT/Overlay.qml" && ok "Overlay root $fn" || bad "missing Overlay $fn"
done

grep -q 'if (!root.themeLive)' "$ROOT/Overlay.qml" && ok "preview snapshots when not live" || bad "missing themeLive snapshot"
grep -q 'clearThemeSession' "$ROOT/Overlay.qml" && ok "revert clears theme session" || bad "missing clearThemeSession"
grep -q 'property string moduleName: "io.github.chris.chroma"' "$ROOT/Overlay.qml" && ok "overlay moduleName declared" || bad "overlay moduleName missing"
grep -q 'hasRevertTarget' "$ROOT/Overlay.qml" && ok "preview refuses without revert target" || bad "missing hasRevertTarget refuse"
grep -q 'AnchorChanges' "$ROOT/Overlay.qml" && ok "HUD uses AnchorChanges" || bad "HUD anchors"
grep -q 'chromad not ready' "$ROOT/Overlay.qml" && ok "theme apply fails closed without chromad" || bad "theme fail-closed"
grep -q 'property var surfaceBorderSpec' "$ROOT/qml/Hud.qml" && ok "Hud surfaceBorderSpec" || bad "Hud still redeclares borderSpec"
grep -q 'property var surfaceBorderSpec' "$ROOT/qml/HelpCard.qml" && ok "HelpCard surfaceBorderSpec" || bad "HelpCard still redeclares borderSpec"
grep -q 'property var surfaceBorderSpec' "$ROOT/qml/Toast.qml" && ok "Toast surfaceBorderSpec" || bad "Toast still redeclares borderSpec"
grep -q 'as ColorMath' "$ROOT/qml/ContrastBadge.qml" && ok "ContrastBadge ColorMath import" || bad "ContrastBadge shadows Color"
if grep -q 'root.ruler.start' "$ROOT/Overlay.qml"; then
  bad "Overlay still uses root.ruler.start"
else
  ok "ruler drag uses ruler.start"
fi
grep -q 'ackOverlayHidden' "$ROOT/Overlay.qml" && ok "overlay hide waits for compositor" || bad "overlay still uses a fixed hide timer"
if grep -q 'interval: 34' "$ROOT/Overlay.qml"; then
  bad "overlay still has 34ms hide timer"
else
  ok "no 34ms hide timer"
fi
if grep -q 'awk_pixel\|#808080' "$ROOT/compat/chromad.sh"; then
  bad "compat still synthesizes #808080"
else
  ok "compat does not synthesize #808080"
fi
grep -q 'request_stop' "$ROOT/compat/chromad.sh" && ok "compat INT/TERM stops serve loop" || bad "compat trap does not stop serve"

printf '%s\n' "$fail failed"
[ "$fail" -eq 0 ]
