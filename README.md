# Chroma

One keystroke, one lens, every question about “what is on my screen.” Zoom loupe, color picker, palette-from-anything → Omarchy theme (with revert), WCAG contrast, pixel ruler, freeze-frame zoom. OCR and QR sit behind feature detection and stay out of the demo.

This is an Omarchy shell plugin (bar-widget + overlay). It runs inside the long-lived `omarchy-shell` process. It does not start a second Quickshell instance.

The catalog already has fragments (Omacolor, Loupe, Theme Colors). Chroma is the unification: color first, then everything else you point at. The closer is **palette → `chroma-preview` theme → one-key revert**.

## Install

```sh
omarchy plugin add <git-url> --enable
```

Then build the helper (optional; grim-only pick mode still gives picker / palette / theme):

```sh
~/.config/omarchy/plugins/io.github.chris.chroma/build.sh
```

Put the droplet on the bar if `--enable` did not:

```sh
omarchy bar put io.github.chris.chroma --section right
```

Reload plugins if the shell was already running:

```sh
omarchy-shell shell rescanPlugins
```

## Usage

Click the droplet, or bind a key. The plugin does **not** write Hyprland config.

```
bind = SUPER, C, exec, omarchy-shell shell summon io.github.chris.chroma '{}'
```

| Key | Action |
|---|---|
| Esc | close (or back out of help / palette / ruler / zoom) |
| Space | pick color (clipboard + history) |
| c | copy HEX |
| p | palette from the active window |
| m | palette from the current monitor |
| t | write `chroma-preview` and apply it |
| u / Backspace | revert to the theme captured before preview |
| r | pixel ruler (drag two points; **px only**) |
| z | freeze-frame zoom |
| + / − | zoom in / out while frozen |
| h | pick history |
| ? | help |
| o | OCR — only if `tesseract` is on PATH |
| q | QR — only if `zbarimg` is on PATH |

Right-click the bar chip opens the lens on the history strip. Overlay chrome uses theme tokens; the loupe ring is accent.

## Palette → theme

`p` hides the overlay for one capture so the lens is not in the shot, runs k-means (k=6, seeded) on the active window clipped to the focused monitor, and fans six swatches. **Make theme** / `t`:

1. Snapshots `~/.config/omarchy/current/theme.name`
2. Writes `~/.config/omarchy/themes/chroma-preview/{colors.toml,hyprland.conf,alacritty.toml}`
3. Applies with **`omarchy-theme-set chroma-preview`** (the same script the shell’s theme switcher uses)
4. Leaves **revert (`u`)** visible

`omarchy-theme-set` copies the theme into `current/theme` and runs `omarchy-theme-set-templates`, so a `colors.toml` is enough for the rest of the desktop. The extra hyprland/alacritty files are a fallback if templates are missing.

## Capture

`chromad --serve` (Rust) talks newline-delimited JSON on stdio:

1. `ext-image-copy-capture-v1` if the compositor advertises it (oneshots)
2. `wlr-screencopy-unstable-v1` for the 128×128 @ 30Hz live region (`overlay_cursor=0`)
3. **grim** — picker/oneshot only. Never a live slideshow. The HUD labels **pick mode**.

`CHROMA_NO_PROTOCOL=1` forces grim. Frames are double-buffered under `/dev/shm/chroma-<pid>/` (or `$XDG_RUNTIME_DIR`) with a query-string cache-bust so `Image { cache: false }` repaints. QML crops the 128px buffer with `sourceClipRect`, so cursor micro-movement does not need a new capture.

Self-capture: loupe sits ~60px off the cursor; one-shot actions hide the overlay for a frame; freeze zoom is a still.

## Honest limitations

- **Keyboard-first.** Click-through input-region masking was not assumed. The overlay takes exclusive keyboard focus. Space picks; the HUD is clickable. This is also the accessibility path.
- **grim-only is pick mode**, not a jittery 8 fps loupe. Palette → theme still works.
- **Ruler is px only.** EDID DPI lies; millimetres and persistent guides are v1.1.
- **OCR/QR are garnish.** Buttons and keys no-op with an install hint (`pacman -S tesseract zbar`) when the binaries are missing. They are not in the 60s demo.
- **Theme apply shells out to `omarchy-theme-set`.** If that script is missing, apply fails and revert is still offered only after a successful snapshot. A broken preview is the worst ending — revert is the whole point.
- **Helper binary.** `bin/chromad` is produced by `build.sh` on the machine (x86_64 and aarch64 Linux). This tree does not ship prebuilt Linux binaries from macOS. Missing binary → `compat/chromad.sh` (grim + python3 k-means).
- **Keybinds are yours.** The plugin never writes `hyprland.conf`.
- **Fractional scale.** Mapping lives in one module (`logical × scale` from `hyprctl -j monitors`). `chromad --calibrate` is the gate; it is not a substitute for a nested Hyprland 1x/1.5x/2x run on the device.
- **No second Quickshell process.** No `omarchy.*` id.

## Settings

Inline on the `shell.json` bar entry (no plugin config file):

```json
{ "id": "io.github.chris.chroma", "loupeOffset": 60, "historyLimit": 24, "zoomDefault": 4 }
```

Pick history is runtime state at `~/.local/state/chroma/history.json`.

## IPC

```sh
omarchy-shell shell summon io.github.chris.chroma '{}'
omarchy-shell shell hide io.github.chris.chroma
omarchy-shell shell call io.github.chris.chroma pick
omarchy-shell shell call io.github.chris.chroma palette
omarchy-shell shell call io.github.chris.chroma revert
omarchy-shell shell call io.github.chris.chroma status
```

The overlay also registers `IpcHandler` target `io.github.chris.chroma`.

## Tests (off-device)

```sh
node tests/run.js
cargo test --manifest-path src/chromad/Cargo.toml
sh tests/cli.test.sh
```

## Remove

```sh
omarchy plugin remove io.github.chris.chroma
```
