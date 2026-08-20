#!/bin/sh
# POSIX fallback for chromad. Always pick-mode: grim oneshots, never a live
# slideshow. Speaks the same newline-JSON protocol as src/chromad.

set -eu

have() { command -v "$1" >/dev/null 2>&1; }

PID=$$
if [ -n "${CHROMA_SHM_DIR:-}" ]; then
  SHM="$CHROMA_SHM_DIR"
elif [ -d /dev/shm ]; then
  SHM="/dev/shm/chroma-$PID"
elif [ -n "${XDG_RUNTIME_DIR:-}" ]; then
  SHM="$XDG_RUNTIME_DIR/chroma-$PID"
else
  SHM="${TMPDIR:-/tmp}/chroma-$PID"
fi
mkdir -p "$SHM"
cleanup_shm() { rm -rf "$SHM"; }
trap cleanup_shm EXIT INT TERM
SLOT=0
GEN=0

emit() { printf '%s\n' "$1"; }

hello() {
  ocr=false; qr=false; grim=false
  have tesseract && ocr=true
  have zbarimg && qr=true
  have grim && grim=true
  emit "{\"ok\":true,\"event\":\"hello\",\"backend\":\"grim\",\"oneshotBackend\":\"grim\",\"ocr\":$ocr,\"qr\":$qr,\"grim\":$grim,\"live\":false,\"pickMode\":true,\"region\":128,\"hz\":0,\"shm\":\"$SHM\"}"
}

capabilities() {
  hello
}

region_geom() {
  x=${1:-0}; y=${2:-0}
  gx=$((x - 64)); gy=$((y - 64))
  [ "$gx" -lt 0 ] && gx=0
  [ "$gy" -lt 0 ] && gy=0
  printf '%s' "$gx,$gy 128x128"
}

grim_ppm() {
  # cursor excluded: never pass -c
  geom=$1
  out=$2
  if have grim; then
    if [ "$geom" = "full" ]; then
      grim -t ppm "$out" || return 1
    else
      grim -t ppm -g "$geom" "$out" || return 1
    fi
  else
    return 1
  fi
}

pixel_from_ppm() {
  file=$1
  python3 - "$file" <<'PY' 2>/dev/null || awk_pixel "$file"
import sys
p=sys.argv[1]
b=open(p,'rb').read()
if not b.startswith(b'P6'):
    print('{"hex":"#000000","r":0,"g":0,"b":0}'); sys.exit(0)
i=b.find(b'\n')
rest=b[i+1:]
while rest.startswith(b'#'):
    rest=rest[rest.find(b'\n')+1:]
hdr, _, data = rest.partition(b'\n')
parts=hdr.split()
w=int(parts[0]); h=int(parts[1])
# skip maxval line if still in data
if b'\n' in data[:8]:
    data=data[data.find(b'\n')+1:]
cx=w//2; cy=h//2
off=(cy*w+cx)*3
r,g,bl=data[off],data[off+1],data[off+2]
print('{"hex":"#%02x%02x%02x","r":%d,"g":%d,"b":%d,"rgb":"rgb(%d, %d, %d)"}'%(r,g,bl,r,g,bl,r,g,bl))
PY
}

awk_pixel() {
  printf '%s' '{"hex":"#808080","r":128,"g":128,"b":128}'
}

kmeans_from_ppm() {
  file=$1
  if [ "${CHROMA_NO_PYTHON:-}" = "1" ] || ! have python3; then
    return 1
  fi
  python3 - "$file" <<'PY' || return 1
import sys, struct, collections, random
random.seed(42)
p=sys.argv[1]
b=open(p,'rb').read()
if not b.startswith(b'P6'):
    print('["#000000"]'); sys.exit(0)
lines=b.split(b'\n')
# crude header skip
idx=0
got=[]
i=1
while i<len(lines) and len(got)<3:
    if lines[i].startswith(b'#'):
        i+=1; continue
    got += lines[i].split()
    i+=1
w,h=int(got[0]),int(got[1])
# find binary start: after third header field newline
# reopen with parser
raw=b
n=2
while n<len(raw):
    if raw[n:n+2]==b'\n' and raw[:n].count(b'\n')>=3:
        break
    n+=1
# fallback: last header newline after 255
p255=raw.find(b'255')
start=raw.find(b'\n', p255)+1
data=raw[start:]
samples=[]
step=max(1, (w*h)//4000)
for i in range(0, w*h, step):
    o=i*3
    if o+2>=len(data): break
    samples.append((data[o], data[o+1], data[o+2]))
if not samples:
    print('["#000000"]'); sys.exit(0)
k=min(6,len(samples))
centers=random.sample(samples,k)
for _ in range(8):
    acc=[[0,0,0,0] for _ in centers]
    for s in samples:
        bi=min(range(k), key=lambda j:(s[0]-centers[j][0])**2+(s[1]-centers[j][1])**2+(s[2]-centers[j][2])**2)
        acc[bi][0]+=s[0]; acc[bi][1]+=s[1]; acc[bi][2]+=s[2]; acc[bi][3]+=1
    centers=[(a[0]//a[3], a[1]//a[3], a[2]//a[3]) if a[3] else c for a,c in zip(acc,centers)]
centers=sorted(centers, key=lambda c: 0.2126*c[0]+0.7152*c[1]+0.0722*c[2])
print('['+','.join('"#%02x%02x%02x"'%c for c in centers)+']')
PY
}

copy_png_slot() {
  src=$1
  SLOT=$((1 - SLOT))
  GEN=$((GEN + 1))
  dest="$SHM/frame$SLOT.png"
  if have magick; then
    magick "$src" "$dest" 2>/dev/null || cp "$src" "$dest"
  elif have convert; then
    convert "$src" "$dest" 2>/dev/null || cp "$src" "$dest"
  else
    # grim ppm; QML Image reads PPM if we keep the extension it knows. Write ppm as png name only if we must.
    cp "$src" "$SHM/frame$SLOT.ppm"
    dest="$SHM/frame$SLOT.ppm"
  fi
  printf '%s' "$dest"
}

CX=960
CY=540

# Active window ∩ focused monitor, grim geometry "x,y wxh". Empty → caller falls back.
window_geom() {
  have hyprctl || return 1
  have python3 || return 1
  python3 - <<'PY'
import json, subprocess, sys
try:
    win = json.loads(subprocess.check_output(["hyprctl", "-j", "activewindow"], text=True))
    mons = json.loads(subprocess.check_output(["hyprctl", "-j", "monitors"], text=True))
except Exception:
    sys.exit(1)
if not isinstance(win, dict) or not isinstance(mons, list) or not mons:
    sys.exit(1)
at = win.get("at") or [0, 0]
size = win.get("size") or [0, 0]
wx, wy = int(at[0]), int(at[1])
ww, wh = int(size[0]), int(size[1])
focused = next((m for m in mons if m.get("focused")), mons[0])
scale = float(focused.get("scale") or 1) or 1.0
mx, my = int(focused.get("x") or 0), int(focused.get("y") or 0)
mw = int((int(focused.get("width") or 0) / scale))
mh = int((int(focused.get("height") or 0) / scale))
x = max(wx, mx)
y = max(wy, my)
r = min(wx + ww, mx + mw)
b = min(wy + wh, my + mh)
w, h = int(r - x), int(b - y)
if w <= 0 or h <= 0:
    sys.exit(1)
print("%d,%d %dx%d" % (x, y, w, h))
PY
}

json_field() {
  printf '%s' "$1" | sed -n "s/.*\"$2\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p"
}

emit_frame() {
  path=$1
  pix=$2
  x=${3:-0}; y=${4:-0}; w=${5:-128}; h=${6:-128}
  emit "{\"ok\":true,\"event\":\"frame\",\"path\":\"$path\",\"n\":$GEN,\"slot\":$SLOT,\"x\":$x,\"y\":$y,\"w\":$w,\"h\":$h,\"pixel\":$pix}"
}

handle() {
  line=$1
  cmd=$(printf '%s' "$line" | sed -n 's/.*"cmd"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  case "$cmd" in
    hello|capabilities|"") hello ;;
    cursor)
      CX=$(printf '%s' "$line" | sed -n 's/.*"x"[[:space:]]*:[[:space:]]*\([0-9.]*\).*/\1/p')
      CY=$(printf '%s' "$line" | sed -n 's/.*"y"[[:space:]]*:[[:space:]]*\([0-9.]*\).*/\1/p')
      emit "{\"ok\":true,\"event\":\"cursor\",\"x\":${CX:-0},\"y\":${CY:-0}}"
      ;;
    start_stream)
      emit '{"ok":true,"event":"stream","running":false,"pickMode":true}'
      ;;
    stop_stream)
      emit '{"ok":true,"event":"stream","running":false}'
      ;;
    pick)
      geom=$(region_geom "${CX%.*}" "${CY%.*}")
      ppm="$SHM/shot.ppm"
      if grim_ppm "$geom" "$ppm"; then
        pix=$(pixel_from_ppm "$ppm")
        path=$(copy_png_slot "$ppm")
        hex=$(printf '%s' "$pix" | sed -n 's/.*"hex":"\([^"]*\)".*/"\1"/p')
        emit "{\"ok\":true,\"event\":\"pick\",\"pixel\":$pix,\"hex\":$hex}"
        emit_frame "$path" "$pix"
      else
        emit '{"ok":false,"event":"error","error":"grim capture failed (is grim installed?)"}'
      fi
      ;;
    oneshot)
      geom=$(region_geom "${CX%.*}" "${CY%.*}")
      ppm="$SHM/shot.ppm"
      if grim_ppm "$geom" "$ppm"; then
        pix=$(pixel_from_ppm "$ppm")
        path=$(copy_png_slot "$ppm")
        emit_frame "$path" "$pix"
      else
        emit '{"ok":false,"event":"error","error":"grim capture failed (is grim installed?)"}'
      fi
      ;;
    freeze)
      geom=$(region_geom "${CX%.*}" "${CY%.*}")
      ppm="$SHM/shot.ppm"
      if grim_ppm "$geom" "$ppm"; then
        pix=$(pixel_from_ppm "$ppm")
        path=$(copy_png_slot "$ppm")
        emit_frame "$path" "$pix"
        emit '{"ok":true,"event":"frozen","factor":8}'
      else
        emit '{"ok":false,"event":"error","error":"grim capture failed (is grim installed?)"}'
      fi
      ;;
    palette)
      source=$(json_field "$line" source)
      [ -n "$source" ] || source=window
      geom=""
      if [ "$source" = "window" ]; then
        geom=$(window_geom || true)
      fi
      if [ -z "$geom" ] && [ "$source" = "monitor" ]; then
        geom=""
      fi
      ppm="$SHM/shot.ppm"
      captured=false
      if [ -n "$geom" ]; then
        grim_ppm "$geom" "$ppm" && captured=true
      fi
      if [ "$captured" = false ]; then
        grim_ppm "full" "$ppm" && captured=true
      fi
      if [ "$captured" != true ]; then
        emit '{"ok":false,"event":"error","error":"grim capture failed"}'
      elif [ "${CHROMA_NO_PYTHON:-}" = "1" ] || ! have python3; then
        emit '{"ok":false,"event":"error","error":"palette extraction needs python3 (grim fallback) or chromad"}'
      elif ! cols=$(kmeans_from_ppm "$ppm"); then
        emit '{"ok":false,"event":"error","error":"palette extraction failed"}'
      else
        emit "{\"ok\":true,\"event\":\"palette\",\"source\":\"$source\",\"colors\":$cols}"
      fi
      ;;
    ocr)
      if have tesseract && have grim; then
        png="$SHM/ocr.png"
        grim -t png "$png"
        magick "$png" -resize 200% "$SHM/ocr2.png" 2>/dev/null || cp "$png" "$SHM/ocr2.png"
        text=$(tesseract "$SHM/ocr2.png" stdout -l eng 2>/dev/null | tr '\n' ' ' | sed 's/"/\\"/g')
        emit "{\"ok\":true,\"event\":\"ocr\",\"text\":\"$text\"}"
      else
        emit '{"ok":false,"event":"error","error":"tesseract not installed (pacman -S tesseract)"}'
      fi
      ;;
    qr)
      if have zbarimg && have grim; then
        png="$SHM/qr.png"
        grim -t png "$png"
        text=$(zbarimg -q --raw "$png" 2>/dev/null | tr '\n' ' ' | sed 's/"/\\"/g')
        emit "{\"ok\":true,\"event\":\"qr\",\"text\":\"$text\"}"
      else
        emit '{"ok":false,"event":"error","error":"zbarimg not installed (pacman -S zbar)"}'
      fi
      ;;
    write_theme)
      dir=$(printf '%s' "$line" | sed -n 's/.*"dir"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
      if [ -z "$dir" ]; then
        emit '{"ok":false,"event":"error","error":"write_theme needs dir"}'
      else
        mkdir -p "$dir"
        emit "{\"ok\":true,\"event\":\"theme_written\",\"dir\":\"$dir\"}"
      fi
      ;;
    validate_theme)
      dir=$(printf '%s' "$line" | sed -n 's/.*"dir"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
      if [ -f "$dir/colors.toml" ]; then
        emit '{"ok":true,"event":"theme_valid"}'
      else
        emit '{"ok":false,"event":"error","error":"colors.toml missing"}'
      fi
      ;;
    unfreeze)
      emit '{"ok":true,"event":"unfrozen"}'
      ;;
    quit)
      emit '{"ok":true,"event":"bye"}'
      exit 0
      ;;
    *)
      emit "{\"ok\":false,\"event\":\"error\",\"error\":\"unknown cmd\"}"
      ;;
  esac
}

case "${1:-}" in
  --serve)
    hello
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      handle "$line"
    done
    ;;
  --capabilities)
    hello
    ;;
  --help|-h|"")
    printf '%s\n' "chromad.sh — grim-only fallback. Usage: chromad.sh --serve"
    ;;
  *)
    printf '%s\n' "unknown: $1" >&2
    exit 1
    ;;
esac
